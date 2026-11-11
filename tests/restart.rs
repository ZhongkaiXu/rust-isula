use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

use risula::grpc::proto::{self, RestartRequest, RestartResponse};
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{Request, Response, Status};

struct FakeRestart {
    calls: Arc<Mutex<Vec<RestartRequest>>>,
}

#[tonic::async_trait]
impl proto::container_service_server::ContainerService for FakeRestart {
    type EventsStream = tokio_stream::Empty<Result<proto::Event, Status>>;
    async fn events(
        &self,
        _: Request<proto::EventsRequest>,
    ) -> Result<Response<Self::EventsStream>, Status> {
        Err(Status::unimplemented("events"))
    }
    type LogsStream = tokio_stream::Empty<Result<proto::LogsResponse, Status>>;
    async fn logs(
        &self,
        _: Request<proto::LogsRequest>,
    ) -> Result<Response<Self::LogsStream>, Status> {
        Err(Status::unimplemented("logs"))
    }

    type CopyFromContainerStream =
        tokio_stream::Empty<Result<proto::CopyFromContainerResponse, Status>>;
    async fn copy_from_container(
        &self,
        _: Request<proto::CopyFromContainerRequest>,
    ) -> Result<Response<Self::CopyFromContainerStream>, Status> {
        Err(Status::unimplemented("copy_from_container"))
    }
    type CopyToContainerStream =
        tokio_stream::Empty<Result<proto::CopyToContainerResponse, Status>>;
    async fn copy_to_container(
        &self,
        _: Request<tonic::Streaming<proto::CopyToContainerRequest>>,
    ) -> Result<Response<Self::CopyToContainerStream>, Status> {
        Err(Status::unimplemented("copy_to_container"))
    }

    type AttachStream = tokio_stream::Empty<Result<proto::AttachResponse, Status>>;
    async fn attach(
        &self,
        _: Request<tonic::Streaming<proto::AttachRequest>>,
    ) -> Result<Response<Self::AttachStream>, Status> {
        Err(Status::unimplemented("attach"))
    }

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

    async fn stop(
        &self,
        _request: Request<proto::StopRequest>,
    ) -> Result<Response<proto::StopResponse>, Status> {
        Err(Status::unimplemented("stop"))
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

    async fn restart(
        &self,
        request: Request<RestartRequest>,
    ) -> Result<Response<RestartResponse>, Status> {
        let request = request.into_inner();
        self.calls.lock().unwrap().push(request.clone());
        if request.id == "missing" {
            return Ok(Response::new(RestartResponse {
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
        Ok(Response::new(RestartResponse {
            id: "resolved-container-id".into(),
            errmsg: if request.id == "warning" {
                "restart warning".into()
            } else {
                String::new()
            },
            ..Default::default()
        }))
    }
}

async fn run_restart(args: &[&str]) -> (Output, Vec<RestartRequest>) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("isulad.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let server_calls = calls.clone();
    let server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(
                proto::container_service_server::ContainerServiceServer::new(FakeRestart {
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
            .args(["restart", "-H", &host])
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
async fn restart_defaults_and_prints_each_original_name() {
    let full_id = "a".repeat(64);
    let names = ["first", "123456789abc", full_id.as_str(), "first"];
    let (output, calls) = run_restart(&names).await;
    assert!(output.status.success());
    assert_eq!(
        calls,
        names
            .iter()
            .map(|name| RestartRequest {
                id: (*name).into(),
                timeout: 10,
            })
            .collect::<Vec<_>>()
    );
    assert_eq!(output.stdout, format!("{}\n", names.join("\n")).as_bytes());
    assert!(output.stderr.is_empty());
}

#[tokio::test]
async fn restart_accepts_time_formats_and_uses_last_value() {
    for (args, expected) in [
        (vec!["-t", "30", "test"], 30),
        (vec!["--time=0", "test"], 0),
        (vec!["-t", "-1", "test"], -1),
        (vec!["-t=-2", "test"], -2),
        (vec!["--time", "2147483647", "test"], i32::MAX),
        (vec!["--time=-2147483648", "test"], i32::MIN),
        (vec!["-t", "  +10", "test"], 10),
        (vec!["-t", "010", "test"], 8),
        (vec!["-t", "0x10", "test"], 16),
        (vec!["-t", "-0X80000000", "test"], i32::MIN),
        (vec!["-t", "20", "--time=7", "test"], 7),
        (vec!["-Dt=2", "test"], 2),
        (vec!["-tD", "2", "test"], 2),
    ] {
        let (output, calls) = run_restart(&args).await;
        assert!(output.status.success(), "{args:?}: {output:?}");
        assert_eq!(calls[0].timeout, expected, "{args:?}");
    }
}

#[tokio::test]
async fn restart_rejects_invalid_options_without_rpc() {
    for args in [
        vec!["-t"],
        vec!["--time=", "test"],
        vec!["-t=", "1", "test"],
        vec!["-t==1", "test"],
        vec!["-t1", "test"],
        vec!["-t-1", "test"],
        vec!["-t", "abc", "test"],
        vec!["-t", "2147483648", "test"],
        vec!["-t", "-2147483649", "test"],
        vec!["-t", "1.5", "test"],
        vec!["-t", "08", "test"],
        vec!["-t", "0x", "test"],
        vec!["-t", "0x+1", "test"],
        vec!["-t", "1 ", "test"],
        vec!["-tbad", "-t1", "test"],
        vec!["--force", "test"],
        vec!["-f", "test"],
        vec!["-a", "test"],
        vec!["-H"],
        vec!["-t", "", "test"],
        vec!["--unknown", "test"],
    ] {
        let (output, calls) = run_restart(&args).await;
        assert_eq!(output.status.code(), Some(125), "{args:?}: {output:?}");
        assert!(calls.is_empty(), "{args:?}");
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}

#[tokio::test]
async fn restart_validates_container_count() {
    for args in [vec![], vec!["test"; 1000]] {
        let (output, calls) = run_restart(&args).await;
        assert_eq!(output.status.code(), Some(125));
        assert!(calls.is_empty());
    }
    let (output, calls) = run_restart(&vec!["test"; 999]).await;
    assert!(output.status.success());
    assert_eq!(calls.len(), 999);
}

#[tokio::test]
async fn restart_continues_after_business_transport_and_empty_name_errors() {
    let (output, calls) = run_restart(&["missing", "", "unavailable", "good"]).await;
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
async fn restart_ignores_message_on_success() {
    let (output, _) = run_restart(&["warning"]).await;
    assert!(output.status.success());
    assert_eq!(output.stdout, b"warning\n");
    assert!(output.stderr.is_empty());
}

#[tokio::test]
async fn restart_waits_longer_than_five_seconds() {
    let (output, calls) = run_restart(&["--time=-1", "slow"]).await;
    assert!(output.status.success(), "{output:?}");
    assert_eq!(calls[0].timeout, -1);
}

#[tokio::test]
async fn restart_help_and_option_terminator() {
    let (output, calls) = run_restart(&["--help"]).await;
    assert!(output.status.success());
    assert!(calls.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(!stdout.contains("--force"));
    assert!(stdout.contains("--time"));
    assert!(stdout.contains("default 10"));
    let (output, calls) = run_restart(&["--", "-name"]).await;
    assert!(output.status.success());
    assert_eq!(calls[0].id, "-name");
}

#[test]
fn restart_reports_connection_failure() {
    let directory = tempfile::tempdir().unwrap();
    let host = format!("unix://{}/missing.sock", directory.path().display());
    let output = Command::new(env!("CARGO_BIN_EXE_risula"))
        .env("ISULAD_HOST", host)
        .args(["restart", "test"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("cannot connect"));
}
