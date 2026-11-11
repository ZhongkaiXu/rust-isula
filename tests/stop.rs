use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

use risula::grpc::proto::{self, StopRequest, StopResponse};
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{Request, Response, Status};

struct FakeStop {
    calls: Arc<Mutex<Vec<StopRequest>>>,
}

#[tonic::async_trait]
impl proto::container_service_server::ContainerService for FakeStop {
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

    async fn stop(&self, request: Request<StopRequest>) -> Result<Response<StopResponse>, Status> {
        let request = request.into_inner();
        self.calls.lock().unwrap().push(request.clone());
        if request.id == "missing" {
            return Ok(Response::new(StopResponse {
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
        Ok(Response::new(StopResponse {
            id: "resolved-container-id".into(),
            errmsg: if request.id == "warning" {
                "stop warning".into()
            } else {
                String::new()
            },
            ..Default::default()
        }))
    }
}

async fn run_stop(args: &[&str]) -> (Output, Vec<StopRequest>) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("isulad.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let server_calls = calls.clone();
    let server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(
                proto::container_service_server::ContainerServiceServer::new(FakeStop {
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
            .args(["stop", "-H", &host])
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
async fn stop_defaults_and_prints_each_original_name() {
    let (output, calls) = run_stop(&["first", "second"]).await;
    assert!(output.status.success());
    assert_eq!(
        calls,
        vec![
            StopRequest {
                id: "first".into(),
                force: false,
                timeout: 10
            },
            StopRequest {
                id: "second".into(),
                force: false,
                timeout: 10
            },
        ]
    );
    assert_eq!(output.stdout, b"first\nsecond\n");
    assert!(output.stderr.is_empty());
}

#[tokio::test]
async fn stop_accepts_time_formats_and_uses_last_value() {
    for (args, expected) in [
        (vec!["-t", "30", "test"], 30),
        (vec!["--time=0", "test"], 0),
        (vec!["-t-1", "test"], -1),
        (vec!["-t=-2", "test"], -2),
        (vec!["--time", "2147483647", "test"], i32::MAX),
        (vec!["--time=-2147483648", "test"], i32::MIN),
        (vec!["-t", "  +10", "test"], 10),
        (vec!["-t", "010", "test"], 8),
        (vec!["-t", "0x10", "test"], 16),
        (vec!["-t", "-0X80000000", "test"], i32::MIN),
        (vec!["-t", "20", "--time=7", "test"], 7),
    ] {
        let (output, calls) = run_stop(&args).await;
        assert!(output.status.success(), "{args:?}: {output:?}");
        assert_eq!(calls[0].timeout, expected, "{args:?}");
        assert!(!calls[0].force);
    }
}

#[tokio::test]
async fn stop_force_overrides_time_in_either_order() {
    for args in [
        vec!["-ft30", "test"],
        vec!["--time=-1", "--force", "test"],
        vec!["--force=True", "test"],
        vec!["-f=1", "test"],
    ] {
        let (output, calls) = run_stop(&args).await;
        assert!(output.status.success(), "{args:?}: {output:?}");
        assert!(calls[0].force);
        assert_eq!(calls[0].timeout, 0);
    }
    for args in [
        vec!["-f", "--force=false", "-t2", "test"],
        vec!["-f=0", "-t2", "test"],
    ] {
        let (output, calls) = run_stop(&args).await;
        assert!(output.status.success());
        assert!(!calls[0].force);
        assert_eq!(calls[0].timeout, 2);
    }
}

#[tokio::test]
async fn stop_rejects_invalid_options_without_rpc() {
    for args in [
        vec!["-t"],
        vec!["--time=", "test"],
        vec!["-t=", "1", "test"],
        vec!["-t==1", "test"],
        vec!["-t", "abc", "test"],
        vec!["-t", "2147483648", "test"],
        vec!["-t", "-2147483649", "test"],
        vec!["-t", "1.5", "test"],
        vec!["-t", "08", "test"],
        vec!["-t", "0x", "test"],
        vec!["-t", "0x+1", "test"],
        vec!["-t", "1 ", "test"],
        vec!["-tbad", "-t1", "test"],
        vec!["--force", "-tbad", "test"],
        vec!["--force=", "test"],
        vec!["--force=yes", "test"],
        vec!["-f=no", "test"],
        vec!["--unknown", "test"],
    ] {
        let (output, calls) = run_stop(&args).await;
        assert_eq!(output.status.code(), Some(125), "{args:?}: {output:?}");
        assert!(calls.is_empty(), "{args:?}");
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}

#[tokio::test]
async fn stop_validates_container_count() {
    for args in [vec![], vec!["test"; 1000]] {
        let (output, calls) = run_stop(&args).await;
        assert_eq!(output.status.code(), Some(125));
        assert!(calls.is_empty());
    }
    let (output, calls) = run_stop(&vec!["test"; 999]).await;
    assert!(output.status.success());
    assert_eq!(calls.len(), 999);
}

#[tokio::test]
async fn stop_continues_after_business_transport_and_empty_name_errors() {
    let (output, calls) = run_stop(&["missing", "", "unavailable", "good"]).await;
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
async fn stop_preserves_warning_on_success() {
    let (output, _) = run_stop(&["warning"]).await;
    assert!(output.status.success());
    assert_eq!(output.stdout, b"warning\n");
    assert_eq!(output.stderr, b"stop warning\n");
}

#[tokio::test]
async fn stop_waits_longer_than_five_seconds() {
    let (output, calls) = run_stop(&["--time=-1", "slow"]).await;
    assert!(output.status.success(), "{output:?}");
    assert_eq!(calls[0].timeout, -1);
}

#[tokio::test]
async fn stop_help_and_option_terminator() {
    let (output, calls) = run_stop(&["--help"]).await;
    assert!(output.status.success());
    assert!(calls.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--force"));
    assert!(stdout.contains("--time"));
    assert!(stdout.contains("default 10"));
    let (output, calls) = run_stop(&["--", "-name"]).await;
    assert!(output.status.success());
    assert_eq!(calls[0].id, "-name");
}

#[test]
fn stop_reports_connection_failure() {
    let directory = tempfile::tempdir().unwrap();
    let host = format!("unix://{}/missing.sock", directory.path().display());
    let output = Command::new(env!("CARGO_BIN_EXE_risula"))
        .env("ISULAD_HOST", host)
        .args(["stop", "test"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("cannot connect"));
}
