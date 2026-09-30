use crate::cli::Invocation;
use crate::grpc;

pub async fn run(command: &Invocation) -> bool {
    let force = command.options.iter().any(|option| option.name == "force");
    let mut success = true;

    for name in &command.args {
        if name == "none" || name == "none:latest" {
            eprintln!("risula rmi: cannot remove '{name}': reserved image name");
            success = false;
            continue;
        }

        match grpc::delete_image(&command.host, name, force).await {
            Ok(()) => println!("Image \"{name}\" removed"),
            Err(error) => {
                eprintln!("risula rmi: {name}: {error}");
                success = false;
            }
        }
    }

    success
}
