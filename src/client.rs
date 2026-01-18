use std::sync::Arc;

use secrecy::SecretString;
use tokio_rustls::rustls;
use tracing::{debug, info};

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

struct Authenticator {
    access_token: SecretString,
}

impl Authenticator {
    fn new(access_token: SecretString) -> Self {
        Authenticator { access_token }
    }
}

impl async_imap::Authenticator for Authenticator {
    type Response = String;
    fn process(&mut self, challenge: &[u8]) -> Self::Response {
        debug!("Server challenge {:?}", challenge);
        "Blah".to_string()
    }
}

#[derive(Debug)]
pub struct StreamFormatter {}
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

    pub async fn connection(
        &mut self,
    ) -> Result<
        async_imap::Session<
            logged_stream::LoggedStream<
                tokio_rustls::client::TlsStream<tokio::net::TcpStream>,
                StreamFormatter,
                logged_stream::DefaultFilter,
                logged_stream::ConsoleLogger,
            >,
        >,
        anyhow::Error,
    > {
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

        let logged_stream = logged_stream::LoggedStream::new(
            tls_stream,
            StreamFormatter {},
            logged_stream::DefaultFilter::default(),
            logged_stream::ConsoleLogger::new_unchecked("debug"),
        );

        let mut client = async_imap::Client::new(logged_stream);
        debug!("Create new imap client");

        let _greeting = client.read_response().await?;

        let _offered_auths = client.run_command_and_check_ok(&"CAPABILITY", None).await?;
        let response = client.read_response().await?.expect("capabilities");
        let response_data = response.parsed();

        info!("response {:?}", response_data);

        let ret = client.authenticate(
            "XOAUTH2",
            Authenticator::new(self.token_manager.access_token().await?),
        );
        match ret.await {
            Ok(x) => Ok(x),
            Err((a, _b)) => Err(a.into()),
        }
    }
}
