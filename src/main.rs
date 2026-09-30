use std::env;
use std::process;

use risula::cli::{self, Action};
use risula::{grpc, images, import, load, login, logout, pull, rmi, search, tag};

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
        Ok(Action::Invoke(command)) if command.name == "info" => {
            match grpc::info(&command.host).await {
                Ok(info) => print_info(&info),
                Err(error) => {
                    eprintln!("risula info: {error}");
                    process::exit(1);
                }
            }
        }
        Ok(Action::Invoke(command)) if command.name == "images" => {
            if let Err(error) = images::run(&command).await {
                eprintln!("risula images: {error}");
                process::exit(1);
            }
        }
        Ok(Action::Invoke(command)) if command.name == "import" => {
            if let Err(error) = import::run(&command).await {
                eprintln!("risula import: {error}");
                process::exit(1);
            }
        }
        Ok(Action::Invoke(command)) if command.name == "login" => {
            if let Err(error) = login::run(&command).await {
                eprintln!("risula login: {error}");
                process::exit(1);
            }
        }
        Ok(Action::Invoke(command)) if command.name == "logout" => {
            if let Err(error) = logout::run(&command).await {
                eprintln!("risula logout: {error}");
                process::exit(1);
            }
        }
        Ok(Action::Invoke(command)) if command.name == "load" => {
            if let Err(error) = load::run(&command).await {
                eprintln!("risula load: {error}");
                process::exit(1);
            }
        }
        Ok(Action::Invoke(command)) if command.name == "pull" => {
            if let Err(error) = pull::run(&command).await {
                eprintln!("risula pull: {error}");
                process::exit(1);
            }
        }
        Ok(Action::Invoke(command)) if command.name == "search" => {
            if let Err(error) = search::run(&command).await {
                eprintln!("risula search: {error}");
                process::exit(1);
            }
        }
        Ok(Action::Invoke(command)) if command.name == "tag" => {
            if let Err(error) = tag::run(&command).await {
                eprintln!("risula tag: {error}");
                process::exit(1);
            }
        }
        Ok(Action::Invoke(command)) if command.name == "rmi" => {
            if !rmi::run(&command).await {
                process::exit(1);
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

fn print_if_present(label: &str, value: &str) {
    if !value.is_empty() {
        println!("{label}: {value}");
    }
}

fn print_info(info: &grpc::proto::InfoResponse) {
    println!("Containers: {}", info.containers_num);
    println!(" Running: {}", info.c_running);
    println!(" Paused: {}", info.c_paused);
    println!(" Stopped: {}", info.c_stopped);
    println!("Images: {}", info.images_num);
    print_if_present("Server Version", &info.version);
    print_if_present("Storage Driver", &info.driver_name);
    for line in info.driver_status.lines() {
        println!(" {line}");
    }
    print_if_present("Logging Driver", &info.logging_driver);
    print_if_present("Cgroup Driver", &info.cgroup_driver);
    print_if_present("Hugetlb Pagesize", &info.huge_page_size);
    print_if_present("Kernel Version", &info.kversion);
    print_if_present("Operating System", &info.operating_system);
    print_if_present("OSType", &info.os_type);
    print_if_present("Architecture", &info.architecture);
    println!("CPUs: {}", info.cpus);
    println!("Total Memory: {} GB", info.total_mem);
    print_if_present("Name", &info.nodename);
    print_if_present("iSulad Root Dir", &info.isulad_root_dir);
    print_if_present("Http Proxy", &info.http_proxy);
    print_if_present("Https Proxy", &info.https_proxy);
    print_if_present("No Proxy", &info.no_proxy);
}
