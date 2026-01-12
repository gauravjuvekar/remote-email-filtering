mod auth;

use remote_email_filtering as ref_;

#[derive(Debug, PartialEq, clap::Subcommand)]
enum Commands {
    /// Run filters
    Filter(Filter),

    /// Login
    Login(Login),
}

#[derive(Debug, PartialEq, clap::Args)]
struct Login {
    provider: auth::Provider,

    /// provider specific config file
    config_json: std::path::PathBuf,

    /// output file with authorized OAuth2 tokens
    authorized_json: std::path::PathBuf,
}

#[derive(Debug, PartialEq, clap::Args)]
struct Filter {
    provider: auth::Provider,

    /// file with authorized OAuth2 tokens from the 'login' command
    authorized_json: std::path::PathBuf,

    /// email address to use. queried using OAuth2 API if unspecified
    #[arg(short, long)]
    email: Option<String>,
}

#[derive(Debug, clap::Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

fn parse_config(
    provider: auth::Provider,
    config_path: std::path::PathBuf,
) -> Result<auth::ProviderConfig, anyhow::Error> {
    let string = std::fs::read_to_string(config_path)?;
    match provider {
        auth::Provider::Google => {
            let config: auth::GoogleProviderConfig = serde_json::from_str(&string)?;
            Ok(auth::ProviderConfig::Google(config))
        }
        auth::Provider::Microsoft => {
            let config: auth::MicrosoftProviderConfig = serde_json::from_str(&string)?;
            Ok(auth::ProviderConfig::Microsoft(config))
        }
    }
}

fn main() -> Result<(), anyhow::Error> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::filter::EnvFilter::from_default_env())
        .init();

    let args = <Cli as clap::Parser>::parse();

    match args.command {
        Commands::Login(login) => {
            let persistable_secret =
                auth::authorize(&parse_config(login.provider, login.config_json)?)?;
            let file = std::fs::File::create(login.authorized_json)?;
            let writer = std::io::BufWriter::new(file);
            serde_json::to_writer(writer, &persistable_secret)?;
            Ok(())
        }
        Commands::Filter(filter) => {
            let f = std::fs::File::open(filter.authorized_json.clone())?;
            let secrets: auth::PersistedSecrets =
                serde_json::from_reader(std::io::BufReader::new(f))?;
            let f = std::fs::OpenOptions::new()
                .write(true)
                .open(filter.authorized_json)?;

            let mut token_manager = auth::TokenManager::new(secrets, Some(f))?;

            let access_token =
                tokio::runtime::Runtime::new()?.block_on(token_manager.access_token())?;

            println!("Got access token {access_token:?}");

            drop(token_manager);

            let my_filter = ref_::actions::Action::Logic(Box::new(ref_::filters::DebugPrint));

            let spec = vec![(
                ref_::types::Folder {
                    path: vec!["INBOX".to_string()],
                },
                vec![my_filter],
            )];
            ref_::filters::mainloop(&spec);
            Ok(())
        }
    }
}
