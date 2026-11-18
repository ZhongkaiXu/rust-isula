use risula::grpc::networks::proto;
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, UnixListener};
use tokio_stream::wrappers::{TcpListenerStream, UnixListenerStream};
use tonic::{Request, Response, Status};
struct FakeNetworks {
    scenario: &'static str,
    calls: Arc<Mutex<Vec<proto::NetworkRemoveRequest>>>,
}
#[tonic::async_trait]
impl proto::network_service_server::NetworkService for FakeNetworks {
    async fn list(
        &self,
        _: Request<proto::NetworkListRequest>,
    ) -> Result<Response<proto::NetworkListResponse>, Status> {
        Err(Status::unimplemented("list"))
    }
    async fn create(
        &self,
        _: Request<proto::NetworkCreateRequest>,
    ) -> Result<Response<proto::NetworkCreateResponse>, Status> {
        Err(Status::unimplemented("create"))
    }
    async fn inspect(
        &self,
        _: Request<proto::NetworkInspectRequest>,
    ) -> Result<Response<proto::NetworkInspectResponse>, Status> {
        Err(Status::unimplemented("inspect"))
    }
    async fn remove(
        &self,
        request: Request<proto::NetworkRemoveRequest>,
    ) -> Result<Response<proto::NetworkRemoveResponse>, Status> {
        let request = request.into_inner();
        let name = request.name.clone();
        self.calls.lock().unwrap().push(request);
        if name == "rpc-error" {
            return Err(Status::unavailable("failed"));
        }
        if self.scenario == "slow" {
            tokio::time::sleep(std::time::Duration::from_secs(6)).await;
        }
        Ok(Response::new(proto::NetworkRemoveResponse {
            name: if name == "empty-response" {
                ""
            } else {
                "daemon-name"
            }
            .into(),
            cc: if name == "missing" { 2 } else { 0 },
            errmsg: if name == "warning" {
                "daemon warning"
            } else {
                ""
            }
            .into(),
        }))
    }
}
async fn run_at(
    args: &[&str],
    scenario: &'static str,
    tcp: bool,
) -> (Output, Vec<proto::NetworkRemoveRequest>) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("daemon.sock");
    let unix = UnixListener::bind(&socket).unwrap();
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let server_calls = calls.clone();
    let server = tokio::spawn(async move {
        let service = proto::network_service_server::NetworkServiceServer::new(FakeNetworks {
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
            .args(["network", "rm", "-H", &host])
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
async fn network_remove_uses_input_names_and_order_on_both_transports() {
    for tcp in [false, true] {
        let (out, calls) = run_at(&["one", "two", "one"], "normal", tcp).await;
        assert!(out.status.success(), "{out:?}");
        assert!(out.stderr.is_empty());
        assert_eq!(out.stdout, b"one\ntwo\none\n");
        assert_eq!(
            calls.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
            vec!["one", "two", "one"]
        );
    }
}
#[tokio::test]
async fn network_remove_stops_after_first_failure() {
    for name in ["missing", "rpc-error", "empty-response", ""] {
        let (out, calls) = run_at(&["one", name, "later"], "normal", false).await;
        assert_eq!(out.status.code(), Some(1));
        assert_eq!(out.stdout, b"one\n");
        assert_eq!(calls.len(), if name.is_empty() { 1 } else { 2 });
        assert!(String::from_utf8(out.stderr)
            .unwrap()
            .contains("Remove network"));
    }
}
#[tokio::test]
async fn network_remove_preserves_warning_and_option_terminator() {
    let (out, calls) = run_at(&["--debug=false", "--", "warning", "-name"], "normal", true).await;
    assert!(out.status.success());
    assert_eq!(out.stdout, b"warning\n-name\n");
    assert_eq!(out.stderr, b"daemon warning\n");
    assert_eq!(calls[1].name, "-name");
}
#[tokio::test]
async fn network_remove_checks_cli_before_rpc() {
    for args in [
        vec![],
        vec!["--unknown"],
        vec!["-f", "one"],
        vec!["--debug=bad", "one"],
        vec!["--host=bad", "one"],
        vec!["one"; 1000],
    ] {
        let (out, calls) = run_at(&args, "normal", false).await;
        assert_eq!(out.status.code(), Some(125), "{out:?}");
        assert!(calls.is_empty());
        assert!(out.stdout.is_empty());
    }
    let (out, calls) = run_at(&["--help"], "normal", false).await;
    assert!(out.status.success());
    assert!(calls.is_empty());
}
#[tokio::test]
async fn network_remove_waits_for_slow_daemon() {
    let (out, _) = run_at(&["one"], "slow", true).await;
    assert!(out.status.success(), "{out:?}");
}
