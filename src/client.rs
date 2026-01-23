use std::sync::Arc;

use secrecy::SecretString;
use tokio_rustls::rustls;
use tracing::debug;

use crate::auth;
use crate::auth::Provider;
pub struct ImapConfig {
    host: &'static str,
    port: u16,
}

fn imap_config(provider: auth::Provider) -> ImapConfig {
    ImapConfig {
        host: match provider {
            Provider::Google => &"imap.gmail.com",
            Provider::Microsoft => &"outlook.office365.com",
        },
        port: match provider {
            Provider::Google | Provider::Microsoft => 993,
        },
    }
}

pub struct ConnectionFactory {
    endpoint: ImapConfig,
    user: String,
    token_manager: auth::TokenManager,
    tls_client_config: tokio_rustls::rustls::ClientConfig,
}

struct SaslCallback {
    user: String,
    access_token: SecretString,
}

struct Authenticator {
    sasl_session: rsasl::prelude::Session,
}

impl rsasl::callback::SessionCallback for SaslCallback {
    fn callback(
        &self,
        session_data: &rsasl::callback::SessionData,
        context: &rsasl::callback::Context,
        request: &mut rsasl::callback::Request,
    ) -> Result<(), rsasl::prelude::SessionError> {
        use rsasl::property::*;
        use secrecy::ExposeSecret;
        request
            .satisfy::<AuthId>(&self.user)?
            .satisfy::<AuthzId>(&self.user)?
            .satisfy::<OAuthBearerToken>(self.access_token.expose_secret())?;
        Ok(())
    }
}

impl Authenticator {
    fn new(sasl_session: rsasl::prelude::Session) -> Self {
        Authenticator { sasl_session }
    }
}

impl async_imap::Authenticator for Authenticator {
    type Response = String;
    fn process(&mut self, challenge: &[u8]) -> Self::Response {
        assert!(self.sasl_session.are_we_first());
        let mut writer: std::vec::Vec<u8> = vec![];
        let _ = self.sasl_session.step(Some(challenge), &mut writer);
        String::from_utf8(writer).expect("valid utf8")
    }
}

#[cfg(feature = "insecure-raw-logging")]
#[derive(Debug)]
pub struct StreamFormatter {}

#[cfg(feature = "insecure-raw-logging")]
impl logged_stream::BufferFormatter for StreamFormatter {
    fn get_separator(&self) -> &str {
        &""
    }

    fn format_byte(&self, byte: &u8) -> String {
        match std::str::from_utf8(&[*byte]) {
            Ok(s) => s.to_string(),
            Err(_) => format!("{}", byte),
        }
    }
}

type UnloggedStream = tokio_rustls::client::TlsStream<tokio::net::TcpStream>;

#[cfg(not(feature = "insecure-raw-logging"))]
type Stream = UnloggedStream;

#[cfg(feature = "insecure-raw-logging")]
type Stream = logged_stream::LoggedStream<
    UnloggedStream,
    StreamFormatter,
    logged_stream::DefaultFilter,
    logged_stream::ConsoleLogger,
>;

impl ConnectionFactory {
    pub fn new(
        provider: auth::Provider,
        user: String,
        token_manager: auth::TokenManager,
    ) -> ConnectionFactory {
        let root_store = rustls::RootCertStore {
            roots: webpki_roots::TLS_SERVER_ROOTS.to_vec(),
        };

        let client_config = tokio_rustls::rustls::ClientConfig::builder()
            .with_root_certificates(root_store)
            .with_no_client_auth();

        ConnectionFactory {
            endpoint: imap_config(provider),
            user,
            token_manager,
            tls_client_config: client_config,
        }
    }

    pub async fn connection(&mut self) -> Result<async_imap::Session<Stream>, anyhow::Error> {
        let raw_stream =
            tokio::net::TcpStream::connect(&(self.endpoint.host, self.endpoint.port)).await?;

        let tls_connector =
            tokio_rustls::TlsConnector::from(Arc::new(self.tls_client_config.clone()));
        let tls_stream = tls_connector
            .connect(
                rustls::pki_types::ServerName::try_from(self.endpoint.host)?,
                raw_stream,
            )
            .await?;

        let stream = {
            #[cfg(feature = "insecure-raw-logging")]
            {
                logged_stream::LoggedStream::new(
                    tls_stream,
                    StreamFormatter {},
                    logged_stream::DefaultFilter::default(),
                    logged_stream::ConsoleLogger::new_unchecked("debug"),
                )
            }

            #[cfg(not(feature = "insecure-raw-logging"))]
            {
                tls_stream
            }
        };

        let mut client = async_imap::Client::new(stream);
        debug!("Create new imap client");

        let _greeting = client.read_response().await?;

        let offered_capabilities = client.capabilities().await?;
        let offered_auths: std::vec::Vec<&rsasl::mechname::Mechname> = offered_capabilities
            .0
            .iter()
            .filter_map(|v| match v {
                async_imap::types::Capability::Atom(_)
                | async_imap::types::Capability::Imap4rev1 => None,
                async_imap::types::Capability::Auth(s) => {
                    match rsasl::mechname::Mechname::parse(s.as_bytes()) {
                        Ok(x) => Some(x),
                        Err(_) => None,
                    }
                }
            })
            .collect();

        debug!("Offered auths: {:?}", offered_auths);

        let sasl_client = {
            use rsasl::mechanisms::oauthbearer::OAUTHBEARER;
            use rsasl::mechanisms::xoauth2::XOAUTH2;
            use rsasl::prelude::*;
            static MECHANISMS: &[Mechanism] = &[OAUTHBEARER, XOAUTH2];
            let config = rsasl::config::SASLConfig::builder()
                .with_registry(Registry::with_mechanisms(MECHANISMS))
                .with_callback(SaslCallback {
                    user: self.user.clone(),
                    access_token: self.token_manager.access_token().await?,
                })?;

            SASLClient::new(config)
        };

        let sasl_session = sasl_client
            .start_suggested(&offered_auths)
            .expect("shared mechanisms");

        let selected_mechanism = sasl_session.get_mechname().as_str().to_string();
        debug!("Attempting login with '{}'", selected_mechanism);

        let ret = client.authenticate(
            selected_mechanism.as_str(),
            Authenticator::new(sasl_session),
        );
        match ret.await {
            Ok(x) => Ok(x),
            Err((a, _b)) => Err(a.into()),
        }
    }
}
