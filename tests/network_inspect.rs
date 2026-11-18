use risula::grpc::networks::proto;
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, UnixListener};
use tokio_stream::wrappers::{TcpListenerStream, UnixListenerStream};
use tonic::{Request, Response, Status};
struct FakeNetworks {
    scenario: &'static str,
    calls: Arc<Mutex<Vec<proto::NetworkInspectRequest>>>,
}
#[tonic::async_trait]
impl proto::network_service_server::NetworkService for FakeNetworks {
    async fn remove(
        &self,
        _: Request<proto::NetworkRemoveRequest>,
    ) -> Result<Response<proto::NetworkRemoveResponse>, Status> {
        Err(Status::unimplemented("remove"))
    }
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
        request: Request<proto::NetworkInspectRequest>,
    ) -> Result<Response<proto::NetworkInspectResponse>, Status> {
        let request = request.into_inner();
        let name = request.name.clone();
        self.calls.lock().unwrap().push(request);
        if name == "rpc-error" {
            return Err(Status::unavailable("failed"));
        }
        if self.scenario == "slow" {
            tokio::time::sleep(std::time::Duration::from_secs(6)).await;
        }
        Ok(Response::new(proto::NetworkInspectResponse {
            network_json: match name.as_str() {
                "empty" => "",
                "invalid" => "{",
                _ => r#"{"name":"test","plugins":[{"type":"bridge"}],"enabled":true}"#,
            }
            .into(),
            cc: if name == "missing" { 2 } else { 0 },
            errmsg: "message".into(),
        }))
    }
}
async fn run_at(
    args: &[&str],
    scenario: &'static str,
    tcp: bool,
) -> (Output, Vec<proto::NetworkInspectRequest>) {
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
            .args(["network", "inspect", "-H", &host])
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
async fn network_inspect_default_and_duplicates_on_both_transports() {
    for tcp in [false, true] {
        let (out, calls) = run_at(&["one", "one"], "normal", tcp).await;
        assert!(out.status.success(), "{out:?}");
        assert!(out.stderr.is_empty());
        assert_eq!(
            calls.iter().map(|c| c.name.as_str()).collect::<Vec<_>>(),
            vec!["one", "one"]
        );
        let object="    {\n        \"name\": test,\n        \"plugins\": [\n            {\n                \"type\": bridge\n            }\n        ],\n        \"enabled\": true\n    }";
        assert_eq!(
            String::from_utf8(out.stdout).unwrap(),
            format!("[\n{object},\n{object}\n]\n")
        );
    }
}
#[tokio::test]
async fn network_inspect_formats_fields_and_arrays() {
    for (format, expected) in [
        ("{{.name}}", "test\n"),
        ("{{json .name}}", "\"test\"\n"),
        ("{{.plugins.type}}", "bridge\n"),
        ("{{.enabled}}", "true\n"),
    ] {
        let (out, _) = run_at(&["-f", format, "one"], "normal", false).await;
        assert!(out.status.success(), "{out:?}");
        assert_eq!(out.stdout, expected.as_bytes());
    }
    let (out, _) = run_at(
        &["-f={{.missing}}", "--format={{.name}}", "one"],
        "normal",
        true,
    )
    .await;
    assert!(out.status.success());
    assert_eq!(out.stdout, b"test\n");
}
#[tokio::test]
async fn network_inspect_stops_on_error_and_preserves_successful_prefix() {
    for name in ["missing", "rpc-error", "empty", "invalid", ""] {
        let (out, calls) = run_at(
            &["--format={{.name}}", "one", name, "later"],
            "normal",
            false,
        )
        .await;
        assert_eq!(out.status.code(), Some(1));
        assert_eq!(out.stdout, b"test\n");
        assert_eq!(calls.len(), if name.is_empty() { 1 } else { 2 });
    }
    let (out, _) = run_at(&["missing"], "normal", false).await;
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(out.stdout, b"[]\n");
    let (out, _) = run_at(&["--format={{.absent}}", "one"], "normal", false).await;
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
}
#[tokio::test]
async fn network_inspect_validates_arguments_before_rpc() {
    for (args, code) in [
        (vec![], 125),
        (vec!["--unknown"], 125),
        (vec!["-f"], 125),
        (vec!["--debug=bad", "one"], 125),
        (vec!["--format=", "one"], 1),
        (vec!["-f", "{{.name}} {{.enabled}}", "one"], 1),
        (vec!["-f", "{{.}}", "one"], 1),
        (vec!["one"; 1000], 125),
    ] {
        let (out, calls) = run_at(&args, "normal", false).await;
        assert_eq!(out.status.code(), Some(code), "{out:?}");
        assert!(calls.is_empty());
        assert!(out.stdout.is_empty());
    }
    let (out, calls) = run_at(&["--help"], "normal", false).await;
    assert!(out.status.success());
    assert!(calls.is_empty());
}
#[tokio::test]
async fn network_inspect_waits_for_slow_daemon() {
    let (out, _) = run_at(&["one"], "slow", true).await;
    assert!(out.status.success(), "{out:?}");
}
