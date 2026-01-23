use remote_email_filtering::*;

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
    email: String,
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
        Commands::Login(args) => {
            let persistable_secret =
                auth::authorize(&parse_config(args.provider, args.config_json)?)?;
            let file = std::fs::File::create(args.authorized_json)?;
            let writer = std::io::BufWriter::new(file);
            serde_json::to_writer(writer, &persistable_secret)?;
            Ok(())
        }
        Commands::Filter(args) => {
            let f = std::fs::File::open(args.authorized_json.clone())?;
            let secrets: auth::PersistedSecrets =
                serde_json::from_reader(std::io::BufReader::new(f))?;
            let f = std::fs::OpenOptions::new()
                .write(true)
                .open(args.authorized_json)?;

            let token_manager = auth::TokenManager::new(secrets, Some(f))?;

            let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();

            let client_factory =
                client::ConnectionFactory::new(args.provider, args.email, token_manager);

            let my_filter = actions::Action::Logic(Box::new(filters::DebugPrint));

            let spec = vec![(
                types::Folder {
                    path: vec!["INBOX".to_string()],
                },
                vec![my_filter],
            )];

            let runtime = tokio::runtime::Runtime::new()?;
            runtime.block_on(filters::mainloop(&spec, client_factory))
        }
    }
}
