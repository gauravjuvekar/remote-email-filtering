use oauth2::TokenResponse;
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tracing::debug;

#[derive(Error, Debug)]
pub enum AuthorizationError {
    #[error("invalid config while parsing '{field}'")]
    ConfigError {
        source: oauth2::url::ParseError,
        field: String,
    },
    #[error("unexpected response: {msg}")]
    ResponseError {
        source: Option<anyhow::Error>,
        msg: String,
    },

    #[error("timed out waiting for user authorization to complete")]
    Timeout,

    #[error("unknown auth failure")]
    Unknown { source: anyhow::Error },
}

#[derive(Clone, Debug, PartialEq, clap::ValueEnum)]
pub enum Provider {
    Google,
    Microsoft,
}

#[derive(Debug, Deserialize)]
pub struct GoogleProviderConfig {
    client_id: String,
    client_secret: SecretString,
    auth_url: String,
    token_url: String,
}

#[derive(Debug, Deserialize)]
pub struct MicrosoftProviderConfig {
    client_id: String,
    devicecode_url: String,
    auth_url: String,
    token_url: String,
}

pub enum ProviderConfig {
    Google(GoogleProviderConfig),
    Microsoft(MicrosoftProviderConfig),
}

pub fn serialize_secret_string<S: serde::Serializer>(
    secret: &SecretString,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    use secrecy::ExposeSecret;
    serializer.serialize_str(secret.expose_secret())
}

pub fn serialize_optional_secret_string<S: serde::Serializer>(
    secret: &Option<SecretString>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match secret {
        Some(s) => serialize_secret_string(s, serializer),
        None => Option::<String>::serialize(&None, serializer),
    }
}

type AccessToken = SecretString;

/// Secrets enough to obtain access tokens from refresh tokens
#[derive(Debug, Serialize, Deserialize)]
pub struct PersistedSecrets {
    client_id: String,
    #[serde(
        skip_serializing_if = "Option::is_none",
        serialize_with = "serialize_optional_secret_string"
    )]
    client_secret: Option<SecretString>,
    #[serde(serialize_with = "serialize_secret_string")]
    refresh_token: SecretString,
    #[serde(serialize_with = "serialize_secret_string")]
    access_token: AccessToken,
    access_token_validity: chrono::DateTime<chrono::Utc>,
    token_url: String,
}

impl secrecy::SerializableSecret for PersistedSecrets {}

fn get_request_fn(
) -> impl Fn(http::Request<Vec<u8>>) -> std::result::Result<oauth2::HttpResponse, reqwest::Error> {
    fn translate_request(
        req: oauth2::HttpRequest,
        client: &reqwest::blocking::Client,
    ) -> reqwest::blocking::Request {
        let (parts, body) = req.into_parts();
        let uri_str: std::string::String = parts.uri.to_string();
        client
            .request(parts.method, uri_str)
            .headers(parts.headers)
            .body(body)
            .build()
            .expect("reqwest client should build")
    }

    fn translate_response(res: reqwest::blocking::Response) -> oauth2::HttpResponse {
        let mut builder = http::response::Builder::new();
        builder = builder.status(res.status()).version(res.version());
        {
            let headers = builder.headers_mut().expect("valid http response");
            for (key, value) in res.headers().iter() {
                headers.append(key.clone(), value.clone());
            }
        }
        let u8_body: Vec<u8> = res.bytes().unwrap().into();
        builder.body(u8_body).unwrap()
    }

    let req_client = reqwest::blocking::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("reqwest client should build");

    move |req: oauth2::HttpRequest| {
        let resp = req_client.execute(translate_request(req, &req_client))?;
        Result::<http::Response<Vec<u8>>, reqwest::Error>::Ok(translate_response(resp))
    }
}

fn get_request_fn_async() -> reqwest::Client {
    reqwest::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("reqwest client shoudl build")
}

pub struct TokenManager {
    secrets: PersistedSecrets,
    storage: Option<std::fs::File>,
    client: oauth2::Client<
        oauth2::StandardErrorResponse<oauth2::basic::BasicErrorResponseType>,
        oauth2::StandardTokenResponse<oauth2::EmptyExtraTokenFields, oauth2::basic::BasicTokenType>,
        oauth2::StandardTokenIntrospectionResponse<
            oauth2::EmptyExtraTokenFields,
            oauth2::basic::BasicTokenType,
        >,
        oauth2::StandardRevocableToken,
        oauth2::StandardErrorResponse<oauth2::RevocationErrorResponseType>,
        oauth2::EndpointNotSet,
        oauth2::EndpointNotSet,
        oauth2::EndpointNotSet,
        oauth2::EndpointNotSet,
        oauth2::EndpointSet,
    >,
}

impl TokenManager {
    pub fn new(
        secrets: PersistedSecrets,
        storage: Option<std::fs::File>,
    ) -> Result<TokenManager, AuthorizationError> {
        let token_url = oauth2::TokenUrl::new(secrets.token_url.clone()).map_err(|e| {
            AuthorizationError::ConfigError {
                source: e,
                field: "token_url".to_string(),
            }
        })?;

        debug!("creating new token manager with url {token_url}");
        debug!(
            "existing token valid till {}",
            secrets.access_token_validity
        );

        let mut client =
            oauth2::basic::BasicClient::new(oauth2::ClientId::new(secrets.client_id.clone()))
                .set_token_uri(token_url);
        if let Some(ref client_secret) = secrets.client_secret {
            use secrecy::ExposeSecret;
            client = client.set_client_secret(oauth2::ClientSecret::new(
                client_secret.expose_secret().to_string(),
            ));
        }

        Ok(TokenManager {
            secrets,
            storage,
            client,
        })
    }

    pub async fn access_token(&mut self) -> Result<AccessToken, anyhow::Error> {
        if self.secrets.access_token_validity < chrono::Utc::now() {
            debug!(
                "refreshing access token that expired at {}",
                self.secrets.access_token_validity
            );
            let client = self.client.clone();
            let refresh_token = {
                use secrecy::ExposeSecret;
                oauth2::RefreshToken::new(self.secrets.refresh_token.expose_secret().to_string())
            };
            let refresh_token_request = client.exchange_refresh_token(&refresh_token);
            let new_token = refresh_token_request
                .request_async(&get_request_fn_async())
                .await
                .map_err(|e| AuthorizationError::ResponseError {
                    source: Some(e.into()),
                    msg: "whlie refreshing an access token".to_string(),
                })?;

            self.secrets.access_token = new_token.access_token().clone().into_secret().into();
            self.secrets.access_token_validity = chrono::Utc::now()
                + new_token
                    .expires_in()
                    .unwrap_or_else(|| std::time::Duration::from_secs(0));

            if let Some(new_refresh) = new_token.refresh_token() {
                self.secrets.refresh_token = new_refresh.clone().into_secret().into();
            }
        }

        Ok(self.secrets.access_token.clone())
    }
}

impl Drop for TokenManager {
    fn drop(&mut self) {
        use std::os::unix::prelude::FileExt;
        if let Some(f) = &self.storage {
            let res = f.write_all_at(
                &serde_json::to_vec(&self.secrets).expect("should serialize"),
                0,
            );
            debug!("serialized latest secrets to {f:?}: {res:?}");
        }
    }
}

pub fn authorize(config: &ProviderConfig) -> Result<PersistedSecrets, AuthorizationError> {
    let auth_url_str;
    let token_url_str;
    match config {
        ProviderConfig::Google(c) => {
            auth_url_str = c.auth_url.clone();
            token_url_str = c.token_url.clone();
        }
        ProviderConfig::Microsoft(c) => {
            auth_url_str = c.auth_url.clone();
            token_url_str = c.token_url.clone();
        }
    };

    let auth_url =
        oauth2::AuthUrl::new(auth_url_str).map_err(|e| AuthorizationError::ConfigError {
            source: e,
            field: "auth_url".to_string(),
        })?;
    let token_url = oauth2::TokenUrl::new(token_url_str.clone()).map_err(|e| {
        AuthorizationError::ConfigError {
            source: e,
            field: "token_url".to_string(),
        }
    })?;

    struct RemainingTimeSecs(std::sync::atomic::AtomicU64);
    impl std::fmt::Display for RemainingTimeSecs {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "Waiting for {} seconds",
                self.0.load(std::sync::atomic::Ordering::Relaxed)
            )
        }
    }

    match config {
        ProviderConfig::Google(c) => {
            // Google uses RFC6749-section 4.1 Authorization Code Grant with RFC8252-section 7.3
            // Lopback Interface Redirection (with PKCE)
            // https://developers.google.com/identity/protocols/oauth2/native-app#redirect-uri_loopback
            use std::net::{SocketAddr, TcpListener};

            // Try both IPv4 and IPv6 loopback address
            let bind_addrs = [
                SocketAddr::from(([127, 0, 0, 1], 0)),
                SocketAddr::from(([0, 0, 0, 0, 0, 0, 0, 1], 0)),
            ];

            let listener = TcpListener::bind(&bind_addrs[..]).expect("should have free ports");
            let bind_addr = listener
                .local_addr()
                .expect("should have started listening");
            listener
                .set_nonblocking(true)
                .expect("listener should be non-blocking");
            let redirect_url_str = format!("http://{}", bind_addr);
            debug!("Listening on {}", redirect_url_str);

            let redirect_url = oauth2::RedirectUrl::new(redirect_url_str.clone())
                .expect("valid redirect url we are already listening on");

            debug!("Setting redirect URL to {}", redirect_url);

            let (pkce_code_challenge, pkce_code_verifier) =
                oauth2::PkceCodeChallenge::new_random_sha256();

            let client = {
                use secrecy::ExposeSecret;
                oauth2::basic::BasicClient::new(oauth2::ClientId::new(c.client_id.clone()))
                    .set_client_secret(oauth2::ClientSecret::new(
                        c.client_secret.expose_secret().to_string(),
                    ))
            }
            .set_auth_uri(auth_url)
            .set_token_uri(token_url)
            .set_redirect_uri(redirect_url);

            let (display_url, csrf_state) = client
                .authorize_url(oauth2::CsrfToken::new_random)
                .add_scope(oauth2::Scope::new("https://mail.google.com".to_string()))
                .add_scope(oauth2::Scope::new(
                    "https://www.googleapis.com/auth/userinfo.email".to_string(),
                ))
                .set_pkce_challenge(pkce_code_challenge)
                .url();

            println!("Visit this URL on this device to authorize:\n\n\t{display_url}\n");

            let (code, state) = {
                let start = std::time::Instant::now();
                let deadline = start + std::time::Duration::from_secs(300);

                let remaining_time = status_line::StatusLine::new(RemainingTimeSecs(
                    std::sync::atomic::AtomicU64::new(
                        (deadline - std::time::Instant::now()).as_secs(),
                    ),
                ));

                let mut stream = None;
                while std::time::Instant::now() < deadline {
                    remaining_time.0.store(
                        (deadline - std::time::Instant::now()).as_secs(),
                        std::sync::atomic::Ordering::Relaxed,
                    );

                    match listener.accept() {
                        Ok((s, _socket_addr)) => {
                            stream = Some(s);
                            break;
                        }
                        Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(std::time::Duration::from_secs(1));
                            continue;
                        }
                        Err(e) => return Err(AuthorizationError::Unknown { source: e.into() }),
                    };
                }

                drop(remaining_time);

                match stream {
                    None => {
                        return Err(AuthorizationError::Timeout);
                    }
                    Some(mut s) => {
                        let mut reader = std::io::BufReader::new(&s);

                        let mut line = String::new();
                        std::io::BufRead::read_line(&mut reader, &mut line).map_err(|e| {
                            AuthorizationError::ResponseError {
                                source: Some(e.into()),
                                msg: "expected a whole line".to_string(),
                            }
                        })?;

                        let get_query = line.split_ascii_whitespace().nth(1).ok_or_else(|| {
                            AuthorizationError::ResponseError {
                                source: None,
                                msg: "expected a 3 part GET line".to_string(),
                            }
                        })?;

                        let get_url = url::Url::parse(&(redirect_url_str.clone() + get_query))
                            .expect("valid GET URL");

                        let mut query_dict = std::collections::HashMap::<String, String>::from_iter(
                            get_url.query_pairs().into_owned(),
                        );

                        let code = query_dict.remove("code").ok_or_else(|| {
                            AuthorizationError::ResponseError {
                                source: None,
                                msg: "'code' GET parameter missing".to_string(),
                            }
                        })?;
                        let state = query_dict.remove("state").ok_or_else(|| {
                            AuthorizationError::ResponseError {
                                source: None,
                                msg: "'state' GET parameter missing".to_string(),
                            }
                        })?;

                        let message = "You can close this page.";
                        let response = format!(
                            concat!(
                                "HTTP/1.1 200 OK\r\n",
                                "content-length: {}\r\n",
                                "\r\n",
                                "{}",
                            ),
                            message.len(),
                            message
                        );
                        std::io::Write::write(&mut s, response.as_bytes()).expect(
                            "hopefully the browser didn't timeout and close the connection",
                        );

                        (code, state)
                    }
                }
            };
            if state != *csrf_state.secret() {
                return Err(AuthorizationError::ResponseError {
                    source: None,
                    msg: "CSRF token mismatch in response".to_string(),
                });
            }

            // Exchange the code for standard oauth2 tokens

            let http_client_fn = get_request_fn();

            let tokens = client
                .exchange_code(oauth2::AuthorizationCode::new(code))
                .set_pkce_verifier(pkce_code_verifier)
                .request(&http_client_fn)
                .map_err(|e| AuthorizationError::Unknown { source: e.into() })?;

            Ok(PersistedSecrets {
                client_id: c.client_id.clone(),
                client_secret: Some(c.client_secret.clone()),
                refresh_token: tokens
                    .refresh_token()
                    .ok_or_else(|| AuthorizationError::ResponseError {
                        source: None,
                        msg: "missing refresh token in response".to_string(),
                    })?
                    .clone()
                    .into_secret()
                    .into(),
                access_token: tokens.access_token().clone().into_secret().into(),
                access_token_validity: chrono::Utc::now()
                    + tokens
                        .expires_in()
                        .unwrap_or_else(|| std::time::Duration::from_secs(0)),
                token_url: token_url_str,
            })
        }

        ProviderConfig::Microsoft(c) => {
            // Microsoft uses RFC8628 OAuth 2.0 Device Authorization Grant
            // https://learn.microsoft.com/en-us/entra/identity-platform/v2-oauth2-device-code

            let devicecode_url_str = c.devicecode_url.clone();
            let devicecode_url =
                oauth2::DeviceAuthorizationUrl::new(devicecode_url_str).map_err(|e| {
                    AuthorizationError::ConfigError {
                        source: e,
                        field: "devicecode_url".to_string(),
                    }
                })?;

            let client =
                oauth2::basic::BasicClient::new(oauth2::ClientId::new(c.client_id.clone()))
                    .set_auth_uri(auth_url)
                    .set_token_uri(token_url)
                    .set_device_authorization_url(devicecode_url);

            let device_auth_response: oauth2::StandardDeviceAuthorizationResponse = client
                .exchange_device_code()
                .add_scope(oauth2::Scope::new(
                    "https://outlook.office.com/IMAP.AccessAsUser.All".to_string(),
                ))
                .add_scope(oauth2::Scope::new("offline_access".to_string()))
                .request(&get_request_fn())
                .map_err(|e| AuthorizationError::ResponseError {
                    source: Some(e.into()),
                    msg: "obtaining device authorization details".to_string(),
                })?;

            let display_url = device_auth_response.verification_uri();
            let user_code = device_auth_response.user_code().clone().into_secret();

            let qr_string = qr2term::generate_qr_string(display_url.to_string())
                .map_err(|e| AuthorizationError::Unknown { source: e.into() })?;

            let qr_string: String = qr_string
                .split_inclusive('\n')
                .map(|l| format!("\t{}", l))
                .collect();

            println!("Open this URL on any device:\n\n\t{}\n", display_url);
            print!("{}", qr_string);
            println!("\nand enter the code: {}\n", user_code,);

            let expiry_duration = std::cmp::min(
                device_auth_response.expires_in(),
                std::time::Duration::from_secs(300),
            );

            let deadline = std::time::Instant::now() + expiry_duration;

            let remaining_time = status_line::StatusLine::new(RemainingTimeSecs(
                std::sync::atomic::AtomicU64::new((deadline - std::time::Instant::now()).as_secs()),
            ));

            let tokens = client
                .exchange_device_access_token(&device_auth_response)
                .request(
                    &get_request_fn(),
                    |d| {
                        std::thread::sleep(d);
                        remaining_time.0.store(
                            (deadline - std::time::Instant::now()).as_secs(),
                            std::sync::atomic::Ordering::Relaxed,
                        );
                    },
                    Some(expiry_duration),
                )
                .map_err(|e| AuthorizationError::ResponseError {
                    source: Some(e.into()),
                    msg: "while waiting for user to complete authorization".to_string(),
                })?;
            drop(remaining_time);

            Ok(PersistedSecrets {
                client_id: c.client_id.clone(),
                client_secret: None,
                refresh_token: tokens
                    .refresh_token()
                    .ok_or_else(|| AuthorizationError::ResponseError {
                        source: None,
                        msg: "missing refresh token in response".to_string(),
                    })?
                    .clone()
                    .into_secret()
                    .into(),
                access_token: tokens.access_token().clone().into_secret().into(),
                access_token_validity: chrono::Utc::now()
                    + tokens
                        .expires_in()
                        .unwrap_or_else(|| std::time::Duration::from_secs(0)),
                token_url: token_url_str,
            })
        }
    }
}
