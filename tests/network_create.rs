use risula::grpc::networks::proto;
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, UnixListener};
use tokio_stream::wrappers::{TcpListenerStream, UnixListenerStream};
use tonic::{Request, Response, Status};
struct FakeNetworks {
    scenario: &'static str,
    calls: Arc<Mutex<Vec<proto::NetworkCreateRequest>>>,
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
        request: Request<proto::NetworkCreateRequest>,
    ) -> Result<Response<proto::NetworkCreateResponse>, Status> {
        self.calls.lock().unwrap().push(request.into_inner());
        if self.scenario == "rpc-error" {
            return Err(Status::unavailable("failed"));
        }
        if self.scenario == "slow" {
            tokio::time::sleep(std::time::Duration::from_secs(6)).await;
        }
        Ok(Response::new(proto::NetworkCreateResponse {
            name: if self.scenario == "empty" {
                ""
            } else {
                "daemon-name"
            }
            .into(),
            cc: if self.scenario == "error" { 2 } else { 0 },
            errmsg: if self.scenario == "warning" {
                "warning from daemon"
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
) -> (Output, Vec<proto::NetworkCreateRequest>) {
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
            .args(["network", "create", "-H", &host])
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
async fn network_create_defaults_and_generated_name_on_both_transports() {
    for tcp in [false, true] {
        let (out, calls) = run_at(&[], "normal", tcp).await;
        assert!(out.status.success(), "{out:?}");
        assert_eq!(out.stdout, b"daemon-name\n");
        assert!(out.stderr.is_empty());
        assert_eq!(
            calls,
            vec![proto::NetworkCreateRequest {
                driver: "bridge".into(),
                ..Default::default()
            }]
        );
    }
}
#[tokio::test]
async fn network_create_options_and_last_values_are_forwarded() {
    let (out, calls) = run_at(
        &[
            "--driver=first",
            "-d=last",
            "--gateway",
            "bad-gateway",
            "--subnet=bad-subnet",
            "--internal",
            "--internal=false",
            "--debug=false",
            "name",
        ],
        "normal",
        true,
    )
    .await;
    assert!(out.status.success());
    assert_eq!(
        calls,
        vec![proto::NetworkCreateRequest {
            name: "name".into(),
            driver: "last".into(),
            gateway: "bad-gateway".into(),
            subnet: "bad-subnet".into(),
            internal: false
        }]
    );
    let (out, calls) = run_at(
        &[
            "--driver=",
            "--gateway=",
            "--subnet=",
            "--internal=true",
            "--",
            "",
        ],
        "normal",
        false,
    )
    .await;
    assert!(out.status.success());
    assert_eq!(
        calls,
        vec![proto::NetworkCreateRequest {
            internal: true,
            ..Default::default()
        }]
    );
}
#[tokio::test]
async fn network_create_checks_byte_length_and_invalid_cli_before_rpc() {
    for (args, code) in [
        (vec!["a", "b"], 125),
        (vec!["--unknown"], 125),
        (vec!["--internal=bad"], 125),
        (vec!["-d"], 125),
        (vec!["--host=bad"], 125),
    ] {
        let (out, calls) = run_at(&args, "normal", false).await;
        assert_eq!(out.status.code(), Some(code), "{out:?}");
        assert!(calls.is_empty());
    }
    for name in ["a".repeat(201), "中".repeat(67)] {
        let (out, calls) = run_at(&[&name], "normal", false).await;
        assert_eq!(out.status.code(), Some(1));
        assert!(calls.is_empty());
    }
    let name = "a".repeat(200);
    let (out, calls) = run_at(&[&name], "normal", false).await;
    assert!(out.status.success());
    assert_eq!(calls[0].name, name);
    let (out, calls) = run_at(&["--help"], "normal", false).await;
    assert!(out.status.success());
    assert!(calls.is_empty());
}
#[tokio::test]
async fn network_create_preserves_warning_and_rejects_errors_or_missing_name() {
    let (out, _) = run_at(&[], "warning", false).await;
    assert!(out.status.success());
    assert_eq!(out.stdout, b"daemon-name\n");
    assert_eq!(out.stderr, b"warning from daemon\n");
    for scenario in ["empty", "error", "rpc-error"] {
        let (out, _) = run_at(&[], scenario, false).await;
        assert_eq!(out.status.code(), Some(1));
        assert!(out.stdout.is_empty());
    }
}
#[tokio::test]
async fn network_create_waits_for_slow_daemon() {
    let (out, _) = run_at(&[], "slow", true).await;
    assert!(out.status.success(), "{out:?}");
}
