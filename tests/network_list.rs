use risula::grpc::networks::proto;
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, UnixListener};
use tokio_stream::wrappers::{TcpListenerStream, UnixListenerStream};
use tonic::{Request, Response, Status};
struct FakeNetworks {
    scenario: &'static str,
    calls: Arc<Mutex<Vec<proto::NetworkListRequest>>>,
}
#[tonic::async_trait]
impl proto::network_service_server::NetworkService for FakeNetworks {
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
    async fn list(
        &self,
        request: Request<proto::NetworkListRequest>,
    ) -> Result<Response<proto::NetworkListResponse>, Status> {
        self.calls.lock().unwrap().push(request.into_inner());
        if self.scenario == "rpc-error" {
            return Err(Status::unavailable("failed"));
        }
        if self.scenario == "slow" {
            tokio::time::sleep(std::time::Duration::from_secs(6)).await;
        }
        let networks = match self.scenario {
            "empty" => vec![],
            "long" => vec![proto::NetworkInfo {
                name: "n".repeat(25),
                version: "v".repeat(16),
                plugins: vec!["a".repeat(20), "b".repeat(20)],
            }],
            _ => vec![
                proto::NetworkInfo {
                    name: "zeta".into(),
                    version: "".into(),
                    plugins: vec![],
                },
                proto::NetworkInfo {
                    name: "alpha".into(),
                    version: "1.0.0".into(),
                    plugins: vec!["bridge".into(), "portmap".into()],
                },
            ],
        };
        Ok(Response::new(proto::NetworkListResponse {
            networks,
            cc: if self.scenario == "error" { 2 } else { 0 },
            errmsg: "server message".into(),
        }))
    }
}
async fn run_at(
    args: &[&str],
    scenario: &'static str,
    tcp: bool,
) -> (Output, Vec<proto::NetworkListRequest>) {
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
            .args(["network", "ls", "-H", &host])
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
async fn network_list_sorts_and_formats_on_both_transports() {
    for tcp in [false, true] {
        let (out, calls) = run_at(&[], "normal", tcp).await;
        assert!(out.status.success(), "{out:?}");
        assert!(out.stderr.is_empty());
        let expected = format!(
            "{:<20} {:<15} {:<30} \n{:<20} {:<15} {:<30} \n{:<20} {:<15} {:<30} \n",
            "NAME",
            "VERSION",
            "PLUGIN",
            "alpha",
            "1.0.0",
            "bridge,portmap",
            "zeta",
            "<none>",
            "<none>"
        );
        assert_eq!(out.stdout, expected.as_bytes());
        assert_eq!(calls, vec![proto::NetworkListRequest::default()]);
    }
}
#[tokio::test]
async fn network_list_expands_columns_including_trailing_plugin_separator() {
    let (out, _) = run_at(&[], "long", false).await;
    assert!(out.status.success());
    let expected = format!(
        "{:<25} {:<16} {:<42} \n{} {} {},{}  \n",
        "NAME",
        "VERSION",
        "PLUGIN",
        "n".repeat(25),
        "v".repeat(16),
        "a".repeat(20),
        "b".repeat(20)
    );
    assert_eq!(out.stdout, expected.as_bytes());
}
#[tokio::test]
async fn network_list_maps_filters_and_boolean_overrides() {
    let (out, calls) = run_at(
        &[
            "--quiet=false",
            "-q=true",
            "--debug=false",
            "-f",
            " Name = first ",
            "--filter",
            "NAME=last=value",
            "--filter",
            " plugin = bridge ",
            "--filter=",
            "--filter",
            "=empty-key",
        ],
        "normal",
        true,
    )
    .await;
    assert!(out.status.success(), "{out:?}");
    assert_eq!(out.stdout, b"alpha\nzeta\n");
    assert_eq!(
        calls[0].filters,
        [
            ("name".into(), "last=value".into()),
            ("plugin".into(), "bridge".into()),
            ("".into(), "empty-key".into())
        ]
        .into()
    );
    let (out, _) = run_at(&["-q", "--quiet=false"], "normal", false).await;
    assert!(out.stdout.starts_with(b"NAME"));
}
#[tokio::test]
async fn network_list_empty_results_and_errors() {
    let (out, _) = run_at(&[], "empty", false).await;
    assert!(out.status.success());
    assert_eq!(
        out.stdout,
        b"NAME                 VERSION         PLUGIN                         \n"
    );
    let (out, _) = run_at(&["-q"], "empty", false).await;
    assert!(out.status.success());
    assert!(out.stdout.is_empty());
    for scenario in ["error", "rpc-error"] {
        let (out, _) = run_at(&[], scenario, false).await;
        assert_eq!(out.status.code(), Some(1));
        assert!(out.stdout.is_empty());
    }
}
#[tokio::test]
async fn network_list_rejects_invalid_args_before_rpc() {
    for (args, code) in [
        (vec!["extra"], 125),
        (vec!["--unknown"], 125),
        (vec!["--quiet=bad"], 125),
        (vec!["-f"], 125),
        (vec!["--filter", "malformed"], 1),
        (vec!["--host=bad"], 125),
    ] {
        let (out, calls) = run_at(&args, "normal", false).await;
        assert_eq!(out.status.code(), Some(code), "{args:?}: {out:?}");
        assert!(calls.is_empty());
    }
    let (out, calls) = run_at(&["--help"], "normal", false).await;
    assert!(out.status.success());
    assert!(calls.is_empty());
    assert!(String::from_utf8(out.stdout).unwrap().contains("--filter"));
}
#[tokio::test]
async fn network_list_waits_for_slow_daemon() {
    let (out, _) = run_at(&[], "slow", true).await;
    assert!(out.status.success(), "{out:?}");
}
