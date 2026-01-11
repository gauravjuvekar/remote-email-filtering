mod auth;

use remote_email_filtering as ref_;

use clap;

#[derive(Debug, PartialEq, clap::Subcommand)]
enum Commands {
    /// Run filters
    Filter,

    /// Login
    Login(Login),
}

#[derive(Debug, PartialEq, clap::Args)]
struct Login {
    provider: auth::Provider,

    /// provider specific config file
    config_json: std::path::PathBuf,

    /// output file with authorized oauth2 tokens
    authorized_json: std::path::PathBuf,
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
            return Ok(auth::ProviderConfig::Google(config));
        }
        auth::Provider::Microsoft => {
            let config: auth::MicrosoftProviderConfig = serde_json::from_str(&string)?;
            return Ok(auth::ProviderConfig::Microsoft(config));
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
            let perstable_secret = auth::authorize(&parse_config(login.provider, login.config_json)?)?;
            let file = std::fs::File::create(login.authorized_json)?;
            let mut writer = std::io::BufWriter::new(file);
            serde_json::to_writer(writer, &perstable_secret)?;
            Ok(())
        },
        Commands::Filter => {
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
