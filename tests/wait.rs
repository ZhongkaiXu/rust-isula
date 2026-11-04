use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

use risula::grpc::proto::{self, WaitRequest, WaitResponse};
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{Request, Response, Status};

struct FakeWait {
    calls: Arc<Mutex<Vec<WaitRequest>>>,
}

#[tonic::async_trait]
impl proto::container_service_server::ContainerService for FakeWait {
    async fn stats(
        &self,
        _: Request<proto::StatsRequest>,
    ) -> Result<Response<proto::StatsResponse>, Status> {
        Err(Status::unimplemented("stats"))
    }

    async fn top(
        &self,
        _: Request<proto::TopRequest>,
    ) -> Result<Response<proto::TopResponse>, Status> {
        Err(Status::unimplemented("top"))
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
        _request: Request<proto::InspectContainerRequest>,
    ) -> Result<Response<proto::InspectContainerResponse>, Status> {
        Err(Status::unimplemented("inspect"))
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
    async fn wait(&self, request: Request<WaitRequest>) -> Result<Response<WaitResponse>, Status> {
        let request = request.into_inner();
        self.calls.lock().unwrap().push(request.clone());
        if request.id == "transport" {
            return Err(Status::unavailable("wait unavailable"));
        }
        if request.id == "slow" {
            tokio::time::sleep(std::time::Duration::from_secs(6)).await;
        }
        Ok(Response::new(WaitResponse {
            cc: if request.id == "missing" || request.id.is_empty() {
                1
            } else {
                0
            },
            errmsg: "No such container".into(),
            exit_code: match request.id.as_str() {
                "first" | "slow" => 7,
                "second" => 42,
                "max" => u32::MAX,
                _ => 0,
            },
        }))
    }
}

async fn run_wait(args: &[&str]) -> (Output, Vec<WaitRequest>) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("isulad.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let server_calls = calls.clone();
    let server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(
                proto::container_service_server::ContainerServiceServer::new(FakeWait {
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
            .args(["wait", "-H", &host])
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
async fn wait_prints_exit_codes_and_succeeds_even_when_containers_fail() {
    let (out, calls) = run_wait(&["first", "second", "max"]).await;
    assert!(out.status.success());
    assert_eq!(out.stdout, b"7\n42\n4294967295\n");
    assert_eq!(
        calls,
        vec![
            WaitRequest {
                id: "first".into(),
                condition: 0
            },
            WaitRequest {
                id: "second".into(),
                condition: 0
            },
            WaitRequest {
                id: "max".into(),
                condition: 0
            },
        ]
    );
}

#[tokio::test]
async fn wait_continues_after_daemon_transport_and_empty_name_errors() {
    let (out, calls) = run_wait(&["missing", "first", "transport", "", "second"]).await;
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(out.stdout, b"7\n42\n");
    assert_eq!(calls.len(), 5);
    let error = String::from_utf8(out.stderr).unwrap();
    assert!(error.contains("missing") && error.contains("No such container"));
    assert!(error.contains("transport") && error.contains("wait unavailable"));
}

#[tokio::test]
async fn wait_has_no_five_second_request_deadline_and_preserves_order() {
    let start = std::time::Instant::now();
    let (out, calls) = run_wait(&["slow", "second"]).await;
    assert!(out.status.success());
    assert!(start.elapsed() >= std::time::Duration::from_secs(6));
    assert_eq!(out.stdout, b"7\n42\n");
    assert_eq!(calls[0].id, "slow");
    assert_eq!(calls[1].id, "second");
}

#[tokio::test]
async fn wait_rejects_arguments_before_rpc() {
    for args in [
        vec![],
        vec!["demo"; 1000],
        vec!["--time", "1", "demo"],
        vec!["--condition", "removed", "demo"],
        vec!["-H"],
    ] {
        let (out, calls) = run_wait(&args).await;
        assert_eq!(out.status.code(), Some(125));
        assert!(calls.is_empty());
    }
}

#[tokio::test]
async fn wait_help_and_option_terminator() {
    let (out, calls) = run_wait(&["--help"]).await;
    assert!(out.status.success());
    assert!(calls.is_empty());
    assert!(String::from_utf8(out.stdout)
        .unwrap()
        .contains("wait [OPTIONS]"));
    let (out, calls) = run_wait(&["--", "--name"]).await;
    assert!(out.status.success());
    assert_eq!(calls[0].id, "--name");
}

#[test]
fn wait_reports_each_connection_failure() {
    let directory = tempfile::tempdir().unwrap();
    let host = format!("unix://{}/absent.sock", directory.path().display());
    let out = Command::new(env!("CARGO_BIN_EXE_risula"))
        .args(["wait", "-H", &host, "first", "second"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    let error = String::from_utf8(out.stderr).unwrap();
    assert!(error.contains("first") && error.contains("second"));
    assert!(out.stdout.is_empty());
}
