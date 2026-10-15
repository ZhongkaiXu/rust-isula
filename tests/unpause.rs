use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

use risula::grpc::proto::{self, ResumeRequest, ResumeResponse};
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{Request, Response, Status};

struct FakeResume {
    calls: Arc<Mutex<Vec<ResumeRequest>>>,
}

#[tonic::async_trait]
impl proto::container_service_server::ContainerService for FakeResume {
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
    async fn wait(
        &self,
        _request: Request<proto::WaitRequest>,
    ) -> Result<Response<proto::WaitResponse>, Status> {
        Err(Status::unimplemented("wait"))
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

    async fn stop(
        &self,
        _request: Request<proto::StopRequest>,
    ) -> Result<Response<proto::StopResponse>, Status> {
        Err(Status::unimplemented("stop"))
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

    async fn resume(
        &self,
        request: Request<ResumeRequest>,
    ) -> Result<Response<ResumeResponse>, Status> {
        let request = request.into_inner();
        self.calls.lock().unwrap().push(request.clone());
        if request.id == "missing" {
            return Ok(Response::new(ResumeResponse {
                cc: 42,
                errmsg: "No such container:missing".into(),
                ..Default::default()
            }));
        }
        if request.id == "unavailable" {
            return Err(Status::unavailable("container service unavailable"));
        }
        if request.id == "slow" {
            tokio::time::sleep(std::time::Duration::from_secs(6)).await;
        }
        Ok(Response::new(ResumeResponse {
            id: "resolved-container-id".into(),
            errmsg: "ignored on success".into(),
            ..Default::default()
        }))
    }
}

async fn run_unpause(args: &[&str]) -> (Output, Vec<ResumeRequest>) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("isulad.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let server_calls = calls.clone();
    let server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(
                proto::container_service_server::ContainerServiceServer::new(FakeResume {
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
            .env("ISULAD_HOST", "unix:///unused.sock")
            .args(["unpause", "-H", &host])
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
async fn unpause_sends_each_name_and_prints_original_input() {
    let (output, calls) = run_unpause(&["first", "second"]).await;
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        calls,
        vec![
            ResumeRequest { id: "first".into() },
            ResumeRequest {
                id: "second".into()
            },
        ]
    );
    assert_eq!(output.stdout, b"first\nsecond\n");
    // The fake response includes a different ID and a success errmsg, both ignored.
    assert!(output.stderr.is_empty());
}

#[tokio::test]
async fn unpause_checks_container_count_before_rpc() {
    for args in [vec![], vec!["test"; 1000]] {
        let (output, calls) = run_unpause(&args).await;
        assert_eq!(output.status.code(), Some(125));
        assert!(calls.is_empty());
        assert!(output.stdout.is_empty());
    }
    let (output, calls) = run_unpause(&vec!["test"; 999]).await;
    assert!(output.status.success());
    assert_eq!(calls.len(), 999);
}

#[tokio::test]
async fn unpause_continues_after_business_transport_and_empty_name_errors() {
    let (output, calls) = run_unpause(&["missing", "", "unavailable", "good"]).await;
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        calls.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
        ["missing", "unavailable", "good"]
    );
    assert_eq!(output.stdout, b"good\n");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("code 42"));
    assert!(stderr.contains("No such container:missing"));
    assert!(stderr.contains("container name cannot be empty"));
    assert!(stderr.contains("container service unavailable"));
}

#[tokio::test]
async fn unpause_rejects_unknown_options_and_missing_host() {
    for args in [
        vec!["-f", "test"],
        vec!["--signal=STOP", "test"],
        vec!["--unknown", "test"],
        vec!["-H"],
        vec!["--host=", "test"],
    ] {
        let (output, calls) = run_unpause(&args).await;
        assert_eq!(output.status.code(), Some(125), "{args:?}: {output:?}");
        assert!(calls.is_empty());
        assert!(output.stdout.is_empty());
    }
}

#[tokio::test]
async fn unpause_waits_for_slow_daemon_response() {
    let (output, calls) = run_unpause(&["slow"]).await;
    assert!(output.status.success(), "{output:?}");
    assert_eq!(calls.len(), 1);
}

#[tokio::test]
async fn unpause_help_and_option_terminator() {
    let (output, calls) = run_unpause(&["--help"]).await;
    assert!(output.status.success());
    assert!(calls.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("unpause [OPTIONS] CONTAINER [CONTAINER...]"));
    assert!(stdout.contains("--host"));
    let (output, calls) = run_unpause(&["--", "-name"]).await;
    assert!(output.status.success());
    assert_eq!(calls[0].id, "-name");
}

#[test]
fn unpause_reports_connection_failure_for_each_container() {
    let directory = tempfile::tempdir().unwrap();
    let host = format!("unix://{}/missing.sock", directory.path().display());
    let output = Command::new(env!("CARGO_BIN_EXE_risula"))
        .env("ISULAD_HOST", host)
        .args(["unpause", "first", "second"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(stderr.matches("cannot connect").count(), 2);
    assert!(stderr.contains("first"));
    assert!(stderr.contains("second"));
}
