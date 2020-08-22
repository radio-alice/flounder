use argh::FromArgs;
use flounder::run_server;

#[derive(FromArgs, PartialEq, Debug)]
/// A command with positional arguments.
struct Arguments {
    #[argh(subcommand)]
    sub: Sub,
}

#[derive(FromArgs, PartialEq, Debug)]
#[argh(subcommand)]
enum Sub {
    Admin(Admin),
    RunServer(RunServer),
}

#[derive(FromArgs, PartialEq, Debug)]
/// First subcommand.
#[argh(subcommand, name = "admin")]
struct Admin {
    #[argh(option)]
    /// not implemented yet
    x: usize,
}

#[derive(FromArgs, PartialEq, Debug)]
/// Run server
#[argh(subcommand, name = "run")]
struct RunServer {
    /// config file path
    #[argh(option, short = 'c', default = "default_config()")]
    config: String,
}

fn default_config() -> String {
    return "flounder.toml".to_string();
}
/// Command line entrypoint
fn main() {
    let arg: Arguments = argh::from_env();
    match arg.sub {
        Sub::RunServer(r) => run_server(r.config),
        _ => Ok(()),
    }
    .ok();
}
