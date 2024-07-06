// Mostly taken from
// https://github.com/ramosbugs/oauth2-rs/blob/main/examples/google.rs

use anyhow::anyhow;
use clap;
use oauth2;
use reqwest;
use serde;
use serde_json as json;
use url;

#[derive(Debug, serde::Deserialize)]
struct Oauth2AuthInputs {
    client_id: String,
    auth_uri: String,
    token_uri: String,
    client_secret: String,
}

#[derive(Debug, serde::Deserialize)]
struct GoogleClientSecret {
    installed: Oauth2AuthInputs,
}

#[derive(Debug, clap::Parser)]
struct Cli {
    google_clients_secret: String,
    output_bearer_token: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = <Cli as clap::Parser>::parse();

    let input = std::fs::File::open(args.google_clients_secret)
        .expect("File should open");

    let mut deserializer =
        json::Deserializer::from_reader(std::io::BufReader::new(input));
    let client_secrets =
        <GoogleClientSecret as serde::Deserialize>::deserialize(
            &mut deserializer,
        )
        .expect("JSON should be correct");

    let oauth2_inputs = client_secrets.installed;
    println!("{}", oauth2_inputs.auth_uri);

    let response_listener = std::net::TcpListener::bind("127.0.0.1:0")
        .expect("should have free ports");
    let redirect_uri = "http://127.0.0.1:".to_string()
        + response_listener
            .local_addr()
            .expect("Should have a bound address")
            .port()
            .to_string()
            .as_ref();
    println!("{redirect_uri}");

    let (pkce_code_challenge, pkce_code_verifier) =
        oauth2::PkceCodeChallenge::new_random_sha256();

    let client = oauth2::basic::BasicClient::new(
        oauth2::ClientId::new(oauth2_inputs.client_id),
        Some(oauth2::ClientSecret::new(oauth2_inputs.client_secret)),
        oauth2::AuthUrl::new(oauth2_inputs.auth_uri)?,
        Some(oauth2::TokenUrl::new(oauth2_inputs.token_uri)?),
    )
    .set_redirect_uri(
        oauth2::RedirectUrl::new(redirect_uri.clone())
            .expect("Correct redirect URI"),
    );

    let (authorize_url, csrf_state) = client
        .authorize_url(oauth2::CsrfToken::new_random)
        .add_scope(oauth2::Scope::new("https://mail.google.com".to_string()))
        .set_pkce_challenge(pkce_code_challenge)
        .url();

    println!("");
    println!("Visit this in a browser on the same device:");
    println!("");
    println!("{authorize_url}");

    let (code, state) = {
        let (mut stream, _socket_addr) =
            response_listener.accept().expect("Single connection");

        let mut reader = std::io::BufReader::new(&stream);
        let mut line = String::new();
        std::io::BufRead::read_line(&mut reader, &mut line)
            .expect("a whole line");

        println!("{line}\n");

        let get_query = line
            .split_ascii_whitespace()
            .nth(1)
            .expect("3 part get line");

        let get_url = url::Url::parse(&(redirect_uri.clone() + &get_query))
            .expect("valid GET URL");

        let mut query_dict =
            std::collections::HashMap::<String, String>::from_iter(
                get_url.query_pairs().into_owned(),
            );

        let code = query_dict.remove("code").expect("code must exist");
        let state = query_dict.remove("state").expect("state must exist");

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
        std::io::Write::write(&mut stream, response.as_bytes())
            .expect("Hopefully didn't timeout");

        (code, state)
    };

    if state != *csrf_state.secret() {
        return Err("Returned state did not match CSRF token".into());
    }

    println!("Returned code is {code}\n");

    let http_client = reqwest::blocking::ClientBuilder::new()
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .expect("reqwest client should build");

    let exec_request = |http_request: oauth2::HttpRequest| {
    };

    let token = client
        .exchange_code(oauth2::AuthorizationCode::new(code))
        .set_pkce_verifier(pkce_code_verifier)
        .request(&oauth2::reqwest::http_client)
        .expect("A refresh token");

    let refresh = oauth2::TokenResponse::refresh_token(&token).unwrap();
    let access = oauth2::TokenResponse::access_token(&token);
    let duration = oauth2::TokenResponse::expires_in(&token).unwrap();

    println!("Refresh token: {}", refresh.secret());
    println!("access_token token: {}", access.secret());
    println!("duration: {}", duration.as_secs());

    Ok(())
}
