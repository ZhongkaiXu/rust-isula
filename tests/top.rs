use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

use risula::grpc::proto::{self, TopRequest, TopResponse};
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{Request, Response, Status};

struct FakeTop {
    calls: Arc<Mutex<Vec<TopRequest>>>,
}

#[tonic::async_trait]
impl proto::container_service_server::ContainerService for FakeTop {
    async fn exec(
        &self,
        _: Request<proto::ExecRequest>,
    ) -> Result<Response<proto::ExecResponse>, Status> {
        Err(Status::unimplemented("exec"))
    }
    type RemoteExecStream = tokio_stream::Empty<Result<proto::RemoteExecResponse, Status>>;
    async fn remote_exec(
        &self,
        _: Request<tonic::Streaming<proto::RemoteExecRequest>>,
    ) -> Result<Response<Self::RemoteExecStream>, Status> {
        Err(Status::unimplemented("remote_exec"))
    }

    async fn stats(
        &self,
        _: Request<proto::StatsRequest>,
    ) -> Result<Response<proto::StatsResponse>, Status> {
        Err(Status::unimplemented("stats"))
    }

    async fn list(
        &self,
        _: Request<proto::ListRequest>,
    ) -> Result<Response<proto::ListResponse>, Status> {
        Err(Status::unimplemented("list"))
    }

    async fn resize(
        &self,
        _: Request<proto::ResizeRequest>,
    ) -> Result<Response<proto::ResizeResponse>, Status> {
        Err(Status::unimplemented("resize"))
    }

    async fn create(
        &self,
        _request: Request<proto::CreateRequest>,
    ) -> Result<Response<proto::CreateResponse>, Status> {
        Err(Status::unimplemented("create"))
    }

    async fn restart(
        &self,
        _request: Request<proto::RestartRequest>,
    ) -> Result<Response<proto::RestartResponse>, Status> {
        Err(Status::unimplemented("restart"))
    }

    async fn start(
        &self,
        _request: Request<proto::StartRequest>,
    ) -> Result<Response<proto::StartResponse>, Status> {
        Err(Status::unimplemented("start"))
    }
    async fn inspect(
        &self,
        request: Request<proto::InspectContainerRequest>,
    ) -> Result<Response<proto::InspectContainerResponse>, Status> {
        let request = request.into_inner();
        assert!(!request.bformat);
        assert_eq!(request.timeout, 120);
        if request.id == "inspect-transport" {
            return Err(Status::unavailable("inspect unavailable"));
        }
        if request.id == "slow-inspect" {
            tokio::time::sleep(std::time::Duration::from_secs(6)).await;
        }
        let json = match request.id.as_str() {
            "stopped" => r#"{"State":{"Running":false}}"#,
            "restarting" => r#"{"State":{"Running":true,"Restarting":true}}"#,
            "bad-json" => "broken",
            "no-state" => "{}",
            _ => r#"{"State":{"Running":true,"Restarting":false}}"#,
        };
        Ok(Response::new(proto::InspectContainerResponse {
            container_json: json.into(),
            cc: u32::from(request.id == "missing" || request.id.is_empty()),
            errmsg: "No such container".into(),
        }))
    }
    type RemoteStartStream = tokio_stream::Empty<Result<proto::RemoteStartResponse, Status>>;
    async fn remote_start(
        &self,
        _request: Request<tonic::Streaming<proto::RemoteStartRequest>>,
    ) -> Result<Response<Self::RemoteStartStream>, Status> {
        Err(Status::unimplemented("remote_start"))
    }

    async fn rename(
        &self,
        _request: Request<proto::RenameRequest>,
    ) -> Result<Response<proto::RenameResponse>, Status> {
        Err(Status::unimplemented("rename"))
    }

    async fn resume(
        &self,
        _request: Request<proto::ResumeRequest>,
    ) -> Result<Response<proto::ResumeResponse>, Status> {
        Err(Status::unimplemented("resume"))
    }

    async fn pause(
        &self,
        _request: Request<proto::PauseRequest>,
    ) -> Result<Response<proto::PauseResponse>, Status> {
        Err(Status::unimplemented("pause"))
    }

    async fn kill(
        &self,
        _request: Request<proto::KillRequest>,
    ) -> Result<Response<proto::KillResponse>, Status> {
        Err(Status::unimplemented("kill"))
    }

    async fn delete(
        &self,
        _request: Request<proto::DeleteRequest>,
    ) -> Result<Response<proto::DeleteResponse>, Status> {
        Err(Status::unimplemented("delete"))
    }

    async fn version(
        &self,
        _request: Request<proto::VersionRequest>,
    ) -> Result<Response<proto::VersionResponse>, Status> {
        Err(Status::unimplemented("version"))
    }

    async fn info(
        &self,
        _request: Request<proto::InfoRequest>,
    ) -> Result<Response<proto::InfoResponse>, Status> {
        Err(Status::unimplemented("info"))
    }

    async fn stop(
        &self,
        _: Request<proto::StopRequest>,
    ) -> Result<Response<proto::StopResponse>, Status> {
        Err(Status::unimplemented("stop"))
    }
    async fn wait(
        &self,
        _: Request<proto::WaitRequest>,
    ) -> Result<Response<proto::WaitResponse>, Status> {
        Err(Status::unimplemented("wait"))
    }
    async fn top(&self, request: Request<TopRequest>) -> Result<Response<TopResponse>, Status> {
        let request = request.into_inner();
        self.calls.lock().unwrap().push(request.clone());
        if request.id == "transport" {
            return Err(Status::unavailable("top unavailable"));
        }
        if request.id == "slow-top" {
            tokio::time::sleep(std::time::Duration::from_secs(6)).await;
        }
        Ok(Response::new(TopResponse {
            titles: if request.id == "no-header" {
                vec![]
            } else {
                b"  PID   COMMAND".to_vec()
            },
            processes: if request.id == "empty" {
                vec![]
            } else {
                vec![b"   42   sleep 300".to_vec(), vec![b' ', 0xff]]
            },
            cc: u32::from(request.id == "error"),
            errmsg: "ps failed".into(),
        }))
    }
}

async fn run_top(args: &[&str]) -> (Output, Vec<TopRequest>) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("isulad.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let server_calls = calls.clone();
    let server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(
                proto::container_service_server::ContainerServiceServer::new(FakeTop {
                    calls: server_calls,
                }),
            )
            .serve_with_incoming(UnixListenerStream::new(listener))
            .await
            .unwrap();
    });
    let host = format!("unix://{}", socket.display());
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let output = tokio::task::spawn_blocking(move || {
        Command::new(env!("CARGO_BIN_EXE_risula"))
            .args(["top", "-H", &host])
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
async fn top_passes_ps_arguments_and_preserves_output_bytes() {
    for args in [
        vec!["demo"],
        vec!["demo", "-eo", "pid,comm", "--sort=pid"],
        vec!["demo", "--help"],
    ] {
        let (out, calls) = run_top(&args).await;
        assert!(out.status.success());
        assert_eq!(
            calls,
            vec![TopRequest {
                id: "demo".into(),
                args: args[1..].iter().map(|s| s.to_string()).collect()
            }]
        );
        assert_eq!(out.stdout, b"  PID   COMMAND\n   42   sleep 300\n \xff\n");
    }
}

#[tokio::test]
async fn top_checks_container_state_before_process_query() {
    for name in [
        "missing",
        "",
        "stopped",
        "restarting",
        "bad-json",
        "inspect-transport",
    ] {
        let (out, calls) = run_top(&[name]).await;
        assert_eq!(out.status.code(), Some(1), "{name}");
        assert!(calls.is_empty());
        assert!(out.stdout.is_empty());
        assert!(!out.stderr.is_empty());
    }
    let (out, calls) = run_top(&["no-state"]).await;
    assert!(out.status.success());
    assert_eq!(calls.len(), 1);
}

#[tokio::test]
async fn top_handles_empty_rows_and_header() {
    let (out, _) = run_top(&["empty"]).await;
    assert!(out.status.success());
    assert!(out.stdout.is_empty());
    let (out, _) = run_top(&["no-header"]).await;
    assert!(out.status.success());
    assert_eq!(out.stdout, b"   42   sleep 300\n \xff\n");
}

#[tokio::test]
async fn top_reports_daemon_and_transport_errors() {
    let (out, _) = run_top(&["error"]).await;
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(out.stdout, b"  PID   COMMAND\n   42   sleep 300\n \xff\n");
    assert!(String::from_utf8(out.stderr).unwrap().contains("ps failed"));
    let (out, _) = run_top(&["transport"]).await;
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    assert!(String::from_utf8(out.stderr)
        .unwrap()
        .contains("top unavailable"));
}

#[tokio::test]
async fn top_validates_cli_and_passes_option_terminator() {
    for args in [vec![], vec!["-o", "pid", "demo"], vec!["-H"]] {
        let (out, calls) = run_top(&args).await;
        assert_eq!(out.status.code(), Some(125));
        assert!(calls.is_empty());
    }
    let (out, calls) = run_top(&["--help"]).await;
    assert!(out.status.success());
    assert!(calls.is_empty());
    let (out, calls) = run_top(&["--", "-name", "-ef"]).await;
    assert!(out.status.success());
    assert_eq!(calls[0].id, "-name");
    assert_eq!(calls[0].args, vec!["-ef"]);
}

#[tokio::test]
async fn top_allows_slow_inspect_and_process_queries() {
    for name in ["slow-inspect", "slow-top"] {
        let (out, _) = run_top(&[name]).await;
        assert!(out.status.success(), "{name}");
    }
}
