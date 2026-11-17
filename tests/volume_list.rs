use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

use risula::grpc::volumes::proto;
use tokio::net::{TcpListener, UnixListener};
use tokio_stream::wrappers::{TcpListenerStream, UnixListenerStream};
use tonic::{Request, Response, Status};

struct FakeVolumes {
    scenario: &'static str,
    calls: Arc<Mutex<usize>>,
}
#[tonic::async_trait]
impl proto::volume_service_server::VolumeService for FakeVolumes {
    async fn prune(
        &self,
        _: Request<proto::PruneVolumeRequest>,
    ) -> Result<Response<proto::PruneVolumeResponse>, Status> {
        Err(Status::unimplemented("prune"))
    }

    async fn remove(
        &self,
        _: Request<proto::RemoveVolumeRequest>,
    ) -> Result<Response<proto::RemoveVolumeResponse>, Status> {
        Err(Status::unimplemented("remove"))
    }
    async fn list(
        &self,
        _: Request<proto::ListVolumeRequest>,
    ) -> Result<Response<proto::ListVolumeResponse>, Status> {
        *self.calls.lock().unwrap() += 1;
        if self.scenario == "rpc-error" {
            return Err(Status::unavailable("transport failed"));
        }
        if self.scenario == "slow" {
            tokio::time::sleep(std::time::Duration::from_secs(6)).await;
        }
        let volumes = if self.scenario == "empty" {
            vec![]
        } else {
            vec![
                proto::Volume {
                    driver: "local".into(),
                    name: "zeta".into(),
                },
                proto::Volume {
                    driver: "driver-with-a-long-name".into(),
                    name: "alpha-long-volume".into(),
                },
                proto::Volume {
                    driver: "本地".into(),
                    name: "卷".into(),
                },
            ]
        };
        Ok(Response::new(proto::ListVolumeResponse {
            volumes,
            cc: if self.scenario == "error" { 2 } else { 0 },
            errmsg: "server message".into(),
        }))
    }
}
async fn run_at(args: &[&str], scenario: &'static str, tcp: bool) -> (Output, usize) {
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
    let output = tokio::task::spawn_blocking(move || {
        Command::new(env!("CARGO_BIN_EXE_risula"))
            .args(["volume", "ls", "-H", &host])
            .args(args)
            .output()
            .unwrap()
    })
    .await
    .unwrap();
    server.abort();
    let seen = *calls.lock().unwrap();
    (output, seen)
}
const TABLE: &str = "DRIVER               VOLUME NAME \nlocal                zeta       \ndriver-with-a-long-name alpha-long-volume \n本地               卷        \n";

#[tokio::test]
async fn volume_list_preserves_order_and_native_byte_widths() {
    for tcp in [false, true] {
        let (out, calls) = run_at(&[], "normal", tcp).await;
        assert!(out.status.success(), "{out:?}");
        assert_eq!(out.stdout, TABLE.as_bytes());
        assert!(out.stderr.is_empty());
        assert_eq!(calls, 1);
    }
}
#[tokio::test]
async fn volume_list_quiet_and_boolean_overrides() {
    for args in [
        vec!["-q"],
        vec!["--quiet"],
        vec!["-q=false", "--quiet=true"],
        vec!["-Dq"],
    ] {
        let (out, _) = run_at(&args, "normal", false).await;
        assert!(out.status.success());
        assert_eq!(out.stdout, "zeta\nalpha-long-volume\n卷\n".as_bytes());
    }
    let (out, _) = run_at(&["-q", "--quiet=false", "--debug=false"], "normal", false).await;
    assert!(out.status.success());
    assert_eq!(out.stdout, TABLE.as_bytes());
}
#[tokio::test]
async fn volume_list_empty_response_and_errors() {
    let (out, _) = run_at(&[], "empty", false).await;
    assert!(out.status.success());
    assert_eq!(out.stdout, b"DRIVER               VOLUME NAME \n");
    let (out, _) = run_at(&["-q"], "empty", true).await;
    assert!(out.status.success());
    assert!(out.stdout.is_empty());
    for scenario in ["error", "rpc-error"] {
        let (out, calls) = run_at(&[], scenario, false).await;
        assert_eq!(out.status.code(), Some(1));
        assert_eq!(calls, 1);
        assert!(out.stdout.is_empty());
        assert!(!out.stderr.is_empty());
    }
}
#[tokio::test]
async fn volume_list_invalid_options_and_positionals_exit_one() {
    for args in [
        vec!["extra"],
        vec!["--filter", "name=x"],
        vec!["--quiet=bad"],
        vec!["-H"],
        vec!["--", "extra"],
        vec!["--host=bad"],
    ] {
        let (out, calls) = run_at(&args, "normal", false).await;
        assert_eq!(
            out.status.code(),
            Some(if args[0] == "--filter" { 125 } else { 1 }),
            "{args:?}: {out:?}"
        );
        assert_eq!(calls, 0);
    }
    let (out, calls) = run_at(&["--help"], "normal", false).await;
    assert!(out.status.success());
    assert_eq!(calls, 0);
    assert!(String::from_utf8(out.stdout).unwrap().contains("--quiet"));
}
#[tokio::test]
async fn volume_list_waits_for_slow_daemon() {
    let (out, _) = run_at(&[], "slow", true).await;
    assert!(out.status.success(), "{out:?}");
    assert_eq!(out.stdout, TABLE.as_bytes());
}
