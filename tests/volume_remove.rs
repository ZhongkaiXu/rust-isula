use risula::grpc::volumes::proto;
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, UnixListener};
use tokio_stream::wrappers::{TcpListenerStream, UnixListenerStream};
use tonic::{Request, Response, Status};

struct FakeVolumes {
    calls: Arc<Mutex<Vec<String>>>,
}
#[tonic::async_trait]
impl proto::volume_service_server::VolumeService for FakeVolumes {
    async fn prune(
        &self,
        _: Request<proto::PruneVolumeRequest>,
    ) -> Result<Response<proto::PruneVolumeResponse>, Status> {
        Err(Status::unimplemented("prune"))
    }

    async fn list(
        &self,
        _: Request<proto::ListVolumeRequest>,
    ) -> Result<Response<proto::ListVolumeResponse>, Status> {
        Err(Status::unimplemented("list"))
    }
    async fn remove(
        &self,
        request: Request<proto::RemoveVolumeRequest>,
    ) -> Result<Response<proto::RemoveVolumeResponse>, Status> {
        let name = request.into_inner().name;
        self.calls.lock().unwrap().push(name.clone());
        if name == "rpc-error" {
            return Err(Status::unavailable("transport failed"));
        }
        if name == "slow" {
            tokio::time::sleep(std::time::Duration::from_secs(6)).await;
        }
        Ok(Response::new(proto::RemoveVolumeResponse {
            cc: if name == "in-use" { 2 } else { 0 },
            errmsg: "volume in use".into(),
        }))
    }
}
async fn run_at(args: &[&str], tcp: bool) -> (Output, Vec<String>) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("daemon.sock");
    let unix = UnixListener::bind(&socket).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let server_calls = calls.clone();
    let server = tokio::spawn(async move {
        let service = proto::volume_service_server::VolumeServiceServer::new(FakeVolumes {
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
            .args(["volume", "rm", "-H", &host])
            .args(args)
            .output()
            .unwrap()
    })
    .await
    .unwrap();
    server.abort();
    let seen = calls.lock().unwrap().clone();
    (output, seen)
}

#[tokio::test]
async fn volume_remove_sends_names_in_order_over_both_transports() {
    for tcp in [false, true] {
        let (out, calls) = run_at(&["--debug=false", "first", "last"], tcp).await;
        assert!(out.status.success(), "{out:?}");
        assert_eq!(out.stdout, b"first\nlast\n");
        assert!(out.stderr.is_empty());
        assert_eq!(calls, ["first", "last"]);
    }
}
#[tokio::test]
async fn volume_remove_continues_after_errors_and_skips_empty_names() {
    let (out, calls) = run_at(&["in-use", "ok", "", "rpc-error", "last"], false).await;
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(out.stdout, b"ok\nlast\n");
    assert_eq!(calls, ["in-use", "ok", "rpc-error", "last"]);
    let err = String::from_utf8(out.stderr).unwrap();
    assert!(err.contains("in-use"));
    assert!(err.contains("Missing volume name"));
    assert!(err.contains("transport failed"));
}
#[tokio::test]
async fn volume_remove_validates_cli_before_rpc() {
    for (args, code) in [
        (vec![], 1),
        (vec!["--force", "demo"], 125),
        (vec!["-f", "demo"], 125),
        (vec!["--debug=bad", "demo"], 1),
        (vec!["-H"], 1),
        (vec!["--host=bad", "demo"], 1),
        (vec!["demo"; 1000], 1),
    ] {
        let (out, calls) = run_at(&args, false).await;
        assert_eq!(out.status.code(), Some(code), "{args:?}: {out:?}");
        assert!(calls.is_empty());
    }
    let (out, calls) = run_at(&["--help"], false).await;
    assert!(out.status.success());
    assert!(calls.is_empty());
}
#[tokio::test]
async fn volume_remove_preserves_duplicate_names_and_option_terminator() {
    let (out, calls) = run_at(&["--", "-literal", "same", "same"], false).await;
    assert!(out.status.success());
    assert_eq!(calls, ["-literal", "same", "same"]);
    assert_eq!(out.stdout, b"-literal\nsame\nsame\n");
}
#[tokio::test]
async fn volume_remove_waits_for_slow_daemon() {
    let (out, _) = run_at(&["slow"], true).await;
    assert!(out.status.success(), "{out:?}");
    assert_eq!(out.stdout, b"slow\n");
}
