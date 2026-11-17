use risula::grpc::volumes::proto;
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, UnixListener};
use tokio_stream::wrappers::{TcpListenerStream, UnixListenerStream};
use tonic::{Request, Response, Status};
const PROMPT:&str="WARNING! This will remove all local volumes not used by at least one container.\nAre you sure you want to continue? [y/N]";
struct FakeVolumes {
    scenario: &'static str,
    calls: Arc<Mutex<usize>>,
}
#[tonic::async_trait]
impl proto::volume_service_server::VolumeService for FakeVolumes {
    async fn list(
        &self,
        _: Request<proto::ListVolumeRequest>,
    ) -> Result<Response<proto::ListVolumeResponse>, Status> {
        Err(Status::unimplemented("list"))
    }
    async fn remove(
        &self,
        _: Request<proto::RemoveVolumeRequest>,
    ) -> Result<Response<proto::RemoveVolumeResponse>, Status> {
        Err(Status::unimplemented("remove"))
    }
    async fn prune(
        &self,
        _: Request<proto::PruneVolumeRequest>,
    ) -> Result<Response<proto::PruneVolumeResponse>, Status> {
        *self.calls.lock().unwrap() += 1;
        if self.scenario == "rpc-error" {
            return Err(Status::unavailable("failed"));
        }
        if self.scenario == "slow" {
            tokio::time::sleep(std::time::Duration::from_secs(6)).await;
        }
        Ok(Response::new(proto::PruneVolumeResponse {
            volumes: if self.scenario == "empty" {
                vec![]
            } else {
                vec!["zeta".into(), "".into(), "alpha".into()]
            },
            cc: if self.scenario == "error" { 2 } else { 0 },
            errmsg: "server message".into(),
        }))
    }
}
async fn run_at(args: &[&str], input: &[u8], scenario: &'static str, tcp: bool) -> (Output, usize) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("daemon.sock");
    let unix = UnixListener::bind(&socket).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let calls = Arc::new(Mutex::new(0));
    let server_calls = calls.clone();
    let server = tokio::spawn(async move {
        let service = proto::volume_service_server::VolumeServiceServer::new(FakeVolumes {
            scenario,
            calls: server_calls,
        });
        let server = tonic::transport::Server::builder().add_service(service);
        if tcp {
            server
                .serve_with_incoming(TcpListenerStream::new(listener))
                .await
                .unwrap();
        } else {
            server
                .serve_with_incoming(UnixListenerStream::new(unix))
                .await
                .unwrap();
        }
    });
    let host = if tcp {
        format!("tcp://{address}")
    } else {
        format!("unix://{}", socket.display())
    };
    let args: Vec<_> = args.iter().map(|s| s.to_string()).collect();
    let input = input.to_vec();
    let output = tokio::task::spawn_blocking(move || {
        use std::io::Write;
        let mut child = Command::new(env!("CARGO_BIN_EXE_risula"))
            .args(["volume", "prune", "-H", &host])
            .args(args)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        let _ = stdin.write_all(&input);
        drop(stdin);
        child.wait_with_output().unwrap()
    })
    .await
    .unwrap();
    server.abort();
    let seen = *calls.lock().unwrap();
    (output, seen)
}

#[tokio::test]
async fn volume_prune_force_skips_prompt_and_preserves_order() {
    for tcp in [false, true] {
        let (out, calls) = run_at(&["-f"], b"", "normal", tcp).await;
        assert!(out.status.success(), "{out:?}");
        assert_eq!(out.stdout, b"zeta\nalpha\n");
        assert!(out.stderr.is_empty());
        assert_eq!(calls, 1);
    }
}
#[tokio::test]
async fn volume_prune_decline_and_eof_never_contact_daemon() {
    for input in [b"".as_slice(), b"n\n", b"\n", b" yes\n", b"N", b"0"] {
        let (out, calls) = run_at(&[], input, "normal", false).await;
        assert!(out.status.success());
        assert_eq!(out.stdout, PROMPT.as_bytes());
        assert_eq!(calls, 0);
    }
}
#[tokio::test]
async fn volume_prune_only_checks_first_character_and_last_force_value() {
    for input in [b"y".as_slice(), b"Y\n", b"yes", b"yanything"] {
        let (out, calls) = run_at(&["-f", "--force=false"], input, "normal", false).await;
        assert!(out.status.success());
        assert_eq!(out.stdout, format!("{PROMPT}zeta\nalpha\n").as_bytes());
        assert_eq!(calls, 1);
    }
    let (out, calls) = run_at(
        &["--force=false", "-f=true", "--debug=false"],
        b"n",
        "normal",
        false,
    )
    .await;
    assert!(out.status.success());
    assert_eq!(out.stdout, b"zeta\nalpha\n");
    assert_eq!(calls, 1);
}
#[tokio::test]
async fn volume_prune_empty_results_and_errors() {
    let (out, calls) = run_at(&["-f"], b"", "empty", false).await;
    assert!(out.status.success());
    assert!(out.stdout.is_empty());
    assert_eq!(calls, 1);
    for scenario in ["error", "rpc-error"] {
        let (out, calls) = run_at(&[], b"y", scenario, false).await;
        assert_eq!(out.status.code(), Some(1));
        assert_eq!(out.stdout, PROMPT.as_bytes());
        assert_eq!(calls, 1);
        assert!(!out.stderr.is_empty());
    }
}
#[tokio::test]
async fn volume_prune_validates_options_before_prompt_or_rpc() {
    for (args, code) in [
        (vec!["extra"], 1),
        (vec!["--filter", "x"], 125),
        (vec!["--force=bad"], 1),
        (vec!["-H"], 1),
        (vec!["--host=bad"], 1),
    ] {
        let (out, calls) = run_at(&args, b"y", "normal", false).await;
        assert_eq!(out.status.code(), Some(code), "{args:?}: {out:?}");
        assert!(out.stdout.is_empty());
        assert_eq!(calls, 0);
    }
}
#[tokio::test]
async fn volume_prune_waits_for_slow_daemon() {
    let (out, calls) = run_at(&["-f"], b"", "slow", true).await;
    assert!(out.status.success(), "{out:?}");
    assert_eq!(calls, 1);
}
