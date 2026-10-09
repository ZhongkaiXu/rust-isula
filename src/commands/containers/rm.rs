use std::fs;
use std::io;
use std::path::Path;

use crate::cli::Invocation;
use crate::grpc;

fn remove_console_dir(root: &Path, name: &str) -> io::Result<()> {
    // The daemon's ID must stay within one container's client console directory.
    if !name.starts_with(|ch: char| ch.is_ascii_alphanumeric())
        || !name
            .bytes()
            .all(|ch| ch.is_ascii_alphanumeric() || b"_.-".contains(&ch))
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "invalid container ID for console cleanup",
        ));
    }
    // remove_dir_all does not follow symlinks, including links inside the directory.
    match fs::remove_dir_all(root.join(name)) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        result => result,
    }
}

// Err is an invalid option; Ok(false) means at least one container failed.
pub async fn run(command: &Invocation) -> Result<bool, String> {
    let mut force = false;
    let mut volumes = false;
    for option in &command.options {
        match option.name {
            "force" => force = crate::cli::parse_bool(option.value.as_deref(), "force")?,
            "volumes" => volumes = crate::cli::parse_bool(option.value.as_deref(), "volumes")?,
            _ => {}
        }
    }

    let mut success = true;
    for name in &command.args {
        if name.is_empty() {
            eprintln!("risula rm: container name cannot be empty");
            success = false;
            continue;
        }
        match grpc::remove_container(&command.host, name, force, volumes).await {
            Ok(response) => {
                let removed = if response.id.is_empty() {
                    name
                } else {
                    &response.id
                };
                // Cleanup failure does not undo a successful daemon removal.
                if let Err(error) = remove_console_dir(Path::new("/var/run/isula"), removed) {
                    eprintln!("risula rm: {removed}: cannot clean console directory: {error}");
                }
                println!("{removed}");
            }
            Err(error) => {
                eprintln!("risula rm: {name}: {error}");
                success = false;
            }
        }
    }
    Ok(success)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn console_cleanup_removes_only_selected_directory_without_following_links() {
        let root = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        let sentinel = outside.path().join("keep");
        fs::write(&sentinel, "keep").unwrap();
        let console = root.path().join("container-id");
        fs::create_dir_all(console.join("exec-session")).unwrap();
        fs::write(console.join("exec-session/out-fifo"), "").unwrap();
        symlink(outside.path(), console.join("link")).unwrap();
        remove_console_dir(root.path(), "container-id").unwrap();
        assert!(!console.exists());
        assert!(sentinel.exists());
        remove_console_dir(root.path(), "container-id").unwrap();

        symlink(outside.path(), &console).unwrap();
        remove_console_dir(root.path(), "container-id").unwrap();
        assert!(fs::symlink_metadata(console).is_err());
        assert!(sentinel.exists());
    }

    #[test]
    fn console_cleanup_rejects_paths_and_reports_errors() {
        let root = tempfile::tempdir().unwrap();
        let sentinel = root.path().join("keep");
        fs::write(&sentinel, "keep").unwrap();
        for name in ["", ".", "..", "../keep", "/tmp", "a/../../keep", "a/b"] {
            let error = remove_console_dir(root.path(), name).unwrap_err();
            assert_eq!(error.kind(), io::ErrorKind::InvalidInput, "{name:?}");
            assert!(sentinel.exists());
        }
        assert!(remove_console_dir(root.path(), "keep").is_err());
        assert!(sentinel.exists());
    }
}
