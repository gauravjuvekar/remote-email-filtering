mod actions;
mod filters;
mod folder;
mod types;

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
    let args = <Cli as clap::Parser>::parse();

    if args.command == Commands::Filter {
        let my_filter = actions::Action::Logic(Box::new(filters::Print {
            some_state: 13,
            message: "const action".to_string(),
        }));

        let spec = vec![(
            types::Folder {
                path: vec!["INBOX".to_string()],
            },
            vec![my_filter],
        )];
        filters::mainloop(&spec)
    }
}
