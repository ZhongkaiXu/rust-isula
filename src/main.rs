use std::env;
use std::process;

use risula::cli::{self, Action};
use risula::grpc;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let env_host = env::var("ISULAD_HOST").ok();

    match cli::parse(&args, env_host.as_deref()) {
        Ok(Action::Help(topic)) => cli::print_help(topic),
        Ok(Action::Version) => println!("Version {}", env!("CARGO_PKG_VERSION")),
        Ok(Action::Invoke(command)) if command.name == "version" => {
            match grpc::version(&command.host).await {
                Ok(server) => {
                    println!("Client:\n  Version:\t{}", env!("CARGO_PKG_VERSION"));
                    println!("\nServer:\n  Version:\t{}", server.version);
                    println!("  Git commit:\t{}", server.git_commit);
                    println!("  Built:\t{}", server.build_time);
                }
                Err(error) => {
                    eprintln!("risula version: {error}");
                    process::exit(1);
                }
            }
        }
        Ok(Action::Invoke(command)) => {
            eprintln!("risula {}: command is not implemented", command.name);
            process::exit(1);
        }
        Err(error) => {
            eprintln!("risula: {}", error.message);
            process::exit(error.exit_code);
        }
    }
}
