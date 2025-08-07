use remote_email_filtering as ref_;

use clap;

#[derive(Debug, PartialEq, clap::Subcommand)]
enum Commands {
    /// Run filters
    Filter,

    /// Login
    Login,
}

#[derive(Debug, clap::Args)]
#[group(required = true, multiple = true)]
struct Auth {
    app_registration: std::path::PathBuf,
    app_auth: std::path::PathBuf,
}

#[derive(Debug, clap::Parser)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    #[clap(flatten)]
    auth: Auth,
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::filter::EnvFilter::from_default_env(),
        )
        .init();

    let args = <Cli as clap::Parser>::parse();

    if args.command == Commands::Filter {
        let my_filter =
            ref_::actions::Action::Logic(Box::new(ref_::filters::DebugPrint));

        let spec = vec![(
            ref_::types::Folder {
                path: vec!["INBOX".to_string()],
            },
            vec![my_filter],
        )];
        ref_::filters::mainloop(&spec)
    }
}
