// Mostly taken from
// https://github.com/ramosbugs/oauth2-rs/blob/main/examples/microsoft_devicecode_tenant_user.rs#L25

use anyhow::{anyhow, Context, Result};
use chrono::{DateTime, Utc};
use clap;
use oauth2;
use reqwest;
use serde;
use serde_json as json;
use std::io::Write;
use url;

#[derive(Debug, PartialEq, clap::Subcommand)]
enum Commands {
    InitialGrant {
        microsoft_client_id: String,
        output_refresh_token: String,
    },
    Refresh {
        in_out_refresh_token: String,
    },
}

#[derive(Debug, serde::Serialize, serde::Deserialize, Clone)]
struct Oauth2FinalOutputs {
    access_token: String,
    approx_valid_till: Option<DateTime<Utc>>,
    client_id: String,
    refresh_token: String,
    scopes: Vec<String>,
    token_uri: String,
}

#[derive(Debug, serde::Deserialize)]
struct Oauth2AuthInputs {
    client_id: String,
    auth_uri: String,
    token_uri: String,
    device_code_uri: String,
}

#[derive(Debug, serde::Deserialize)]
struct MicrosoftClientSecret {
    installed: Oauth2AuthInputs,
}

#[derive(Debug, clap::Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

fn get_request_client() -> impl Fn(
    http::Request<Vec<u8>>,
) -> std::result::Result<
    oauth2::HttpResponse,
    reqwest::Error,
> {
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
            .expect("should build")
    }

    fn translate_response(
        res: reqwest::blocking::Response,
    ) -> oauth2::HttpResponse {
        let mut builder = http::response::Builder::new();
        builder = builder.status(res.status()).version(res.version());
        {
            let headers = builder.headers_mut().unwrap();
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

    let http_client_fn = move |req: oauth2::HttpRequest| {
        let resp = req_client.execute(translate_request(req, &req_client))?;
        Result::<http::Response<Vec<u8>>, reqwest::Error>::Ok(
            translate_response(resp),
        )
    };

    http_client_fn
}

fn initial_grant_flow(
    in_client_secrets: String,
    out_secrets: String,
) -> Result<Oauth2FinalOutputs> {
    let input =
        std::fs::File::open(in_client_secrets).expect("File should open");

    let mut deserializer =
        json::Deserializer::from_reader(std::io::BufReader::new(input));
    let client_secrets =
        <MicrosoftClientSecret as serde::Deserialize>::deserialize(
            &mut deserializer,
        )
        .expect("JSON should be correct");

    let oauth2_inputs = client_secrets.installed;
    println!("{}", oauth2_inputs.auth_uri);

    let client = oauth2::basic::BasicClient::new(oauth2::ClientId::new(
        oauth2_inputs.client_id.clone(),
    ))
    .set_auth_uri(oauth2::AuthUrl::new(oauth2_inputs.auth_uri)?)
    .set_token_uri(oauth2::TokenUrl::new(oauth2_inputs.token_uri.clone())?)
    .set_device_authorization_url(oauth2::DeviceAuthorizationUrl::new(oauth2_inputs.device_code_uri.clone())?);

    let device_auth_init_response: oauth2::StandardDeviceAuthorizationResponse = client.exchange_device_code()
        .add_scope(oauth2::Scope::new("https://outlook.office.com/IMAP.AccessAsUser.All".to_string()))
        .add_scope(oauth2::Scope::new("offline_access".to_string()))
        .request(&get_request_client())?;

    println!(
        "Open this URL in your browser:\n\t{}\nand enter the code: {}",
        device_auth_init_response.verification_uri().to_string(),
        device_auth_init_response.user_code().secret().to_string()
    );

    let http_client_fn = get_request_client();

    let token = client
        .exchange_device_access_token(&device_auth_init_response)
        .request(&http_client_fn, std::thread::sleep, None)
        .expect("A refresh token");

    let refresh = oauth2::TokenResponse::refresh_token(&token)
        .expect("Should also get a refresh token");
    let access = oauth2::TokenResponse::access_token(&token);
    let valid_till = match oauth2::TokenResponse::expires_in(&token) {
        Some(d) => Some(Utc::now() + d),
        None => None,
    };
    let scopes: Vec<String> = oauth2::TokenResponse::scopes(&token)
        .unwrap()
        .iter()
        .map(|scope| scope.as_str().to_string())
        .collect();

    println!("Refresh token: {}", refresh.secret());
    println!("access_token token: {}", access.secret());
    println!("valid_till: {:?}", valid_till.unwrap());

    let final_outputs = Oauth2FinalOutputs {
        client_id: oauth2_inputs.client_id,
        access_token: access.secret().to_string(),
        approx_valid_till: valid_till,
        refresh_token: refresh.secret().to_string(),
        scopes,
        token_uri: oauth2_inputs.token_uri,
    };

    let mut output = std::fs::File::create(std::path::Path::new(&out_secrets))
        .expect("File should open");

    let final_json = json::to_string(&final_outputs)?;
    output.write_all(final_json.as_bytes())?;

    Ok(final_outputs)
}

fn refresh_token_flow(
    in_out_refresh_token: String,
) -> Result<Oauth2FinalOutputs> {
    let input =
        std::fs::File::open(&in_out_refresh_token).expect("File should open");

    let mut deserializer =
        json::Deserializer::from_reader(std::io::BufReader::new(input));
    let original_token =
        <Oauth2FinalOutputs as serde::Deserialize>::deserialize(
            &mut deserializer,
        )
        .expect("JSON should be correct");

    let client = oauth2::basic::BasicClient::new(oauth2::ClientId::new(
        original_token.client_id.clone(),
    ))
    .set_token_uri(oauth2::TokenUrl::new(original_token.token_uri.clone())?);

    let refresh_token =
        oauth2::RefreshToken::new(original_token.refresh_token.clone());
    let refresh_token_request = client.exchange_refresh_token(&refresh_token);

    let http_client_fn = get_request_client();
    let new_token = refresh_token_request
        .request(&http_client_fn)
        .expect("new refresh token");

    let mut final_outputs = original_token.clone();
    final_outputs.access_token =
        oauth2::TokenResponse::access_token(&new_token)
            .secret()
            .to_string();

    final_outputs.scopes = oauth2::TokenResponse::scopes(&new_token)
        .unwrap()
        .iter()
        .map(|scope| scope.as_str().to_string())
        .collect();

    final_outputs.approx_valid_till =
        match oauth2::TokenResponse::expires_in(&new_token) {
            Some(d) => Some(Utc::now() + d),
            None => None,
        };

    let mut output =
        std::fs::File::create(std::path::Path::new(&in_out_refresh_token))
            .expect("File should open");

    let final_json = json::to_string(&final_outputs)?;
    output.write_all(final_json.as_bytes())?;

    Ok(final_outputs)
}

fn main() -> Result<()> {
    let args = <Cli as clap::Parser>::parse();

    let _ = match args.command {
        Commands::InitialGrant {
            microsoft_client_id,
            output_refresh_token,
        } => initial_grant_flow(microsoft_client_id, output_refresh_token),

        Commands::Refresh {
            in_out_refresh_token,
        } => refresh_token_flow(in_out_refresh_token),
    };

    Ok(())
}
