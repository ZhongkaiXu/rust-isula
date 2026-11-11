use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

use risula::grpc::proto::{self, RenameRequest, RenameResponse};
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{Request, Response, Status};

struct FakeRename {
    calls: Arc<Mutex<Vec<RenameRequest>>>,
}

#[tonic::async_trait]
impl proto::container_service_server::ContainerService for FakeRename {
    async fn export(
        &self,
        _: Request<proto::ExportRequest>,
    ) -> Result<Response<proto::ExportResponse>, Status> {
        Err(Status::unimplemented("export"))
    }
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

    async fn rename(
        &self,
        request: Request<RenameRequest>,
    ) -> Result<Response<RenameResponse>, Status> {
        let request = request.into_inner();
        self.calls.lock().unwrap().push(request.clone());
        if request.oldname == "missing" {
            return Ok(Response::new(RenameResponse {
                cc: 42,
                errmsg: "No such container:missing".into(),
                ..Default::default()
            }));
        }
        if request.oldname == "unavailable" {
            return Err(Status::unavailable("container service unavailable"));
        }
        if request.oldname == "slow" {
            tokio::time::sleep(std::time::Duration::from_secs(6)).await;
        }
        Ok(Response::new(RenameResponse {
            id: "resolved-container-id".into(),
            errmsg: "ignored on success".into(),
            ..Default::default()
        }))
    }
}

async fn run_rename(args: &[&str]) -> (Output, Vec<RenameRequest>) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("isulad.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let server_calls = calls.clone();
    let server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(
                proto::container_service_server::ContainerServiceServer::new(FakeRename {
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
            .args(["rename", "-H", &host])
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
async fn rename_sends_both_names_and_succeeds_silently() {
    for old_name in ["old-name", "0123456789ab", &"a".repeat(64)] {
        let (output, calls) = run_rename(&[old_name, "new-name"]).await;
        assert!(output.status.success(), "{output:?}");
        assert_eq!(
            calls,
            vec![RenameRequest {
                oldname: old_name.into(),
                newname: "new-name".into(),
            }]
        );
        assert!(output.stdout.is_empty());
        assert!(output.stderr.is_empty());
    }
}

#[tokio::test]
async fn rename_validates_argument_count_and_empty_names_without_rpc() {
    for args in [
        vec![],
        vec!["old"],
        vec!["old", "new", "extra"],
        vec!["", "new"],
        vec!["old", ""],
        vec!["", ""],
    ] {
        let (output, calls) = run_rename(&args).await;
        assert_eq!(output.status.code(), Some(1), "{args:?}: {output:?}");
        assert!(calls.is_empty());
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}

#[tokio::test]
async fn rename_leaves_name_rules_and_conflicts_to_daemon() {
    for (old, new) in [
        ("same", "same"),
        ("old", "bad/name"),
        ("old", "has space"),
        ("old", "-new"),
        ("old", "valid_name.2"),
    ] {
        let (output, calls) = run_rename(&[old, new]).await;
        // The fake daemon accepts these: the client must forward without normalization.
        assert!(output.status.success());
        assert_eq!(
            calls[0],
            RenameRequest {
                oldname: old.into(),
                newname: new.into()
            }
        );
    }
}

#[tokio::test]
async fn rename_reports_business_and_transport_errors() {
    for (name, message) in [
        ("missing", "No such container:missing"),
        ("unavailable", "container service unavailable"),
    ] {
        let (output, calls) = run_rename(&[name, "new"]).await;
        assert_eq!(output.status.code(), Some(1));
        assert_eq!(calls.len(), 1);
        assert!(output.stdout.is_empty());
        let stderr = String::from_utf8(output.stderr).unwrap();
        assert!(stderr.contains(message));
        if name == "missing" {
            assert!(stderr.contains("code 42"));
        }
    }
}

#[tokio::test]
async fn rename_option_errors_help_and_terminator() {
    for args in [
        vec!["--unknown", "old", "new"],
        vec!["-f", "old", "new"],
        vec!["-H"],
        vec!["--host=", "old", "new"],
    ] {
        let (output, calls) = run_rename(&args).await;
        assert_eq!(output.status.code(), Some(125), "{args:?}: {output:?}");
        assert!(calls.is_empty());
    }
    let (output, calls) = run_rename(&["--help"]).await;
    assert!(output.status.success());
    assert!(calls.is_empty());
    assert!(String::from_utf8(output.stdout)
        .unwrap()
        .contains("rename [OPTIONS] OLD_NAME NEW_NAME"));
    let (output, calls) = run_rename(&["--", "-old", "new"]).await;
    assert!(output.status.success());
    assert_eq!(calls[0].oldname, "-old");
}

#[tokio::test]
async fn rename_waits_for_slow_daemon_response() {
    let (output, calls) = run_rename(&["slow", "new"]).await;
    assert!(output.status.success(), "{output:?}");
    assert_eq!(calls.len(), 1);
}

#[test]
fn rename_reports_connection_failure() {
    let directory = tempfile::tempdir().unwrap();
    let output = Command::new(env!("CARGO_BIN_EXE_risula"))
        .env(
            "ISULAD_HOST",
            format!("unix://{}/missing.sock", directory.path().display()),
        )
        .args(["rename", "old", "new"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("cannot connect"));
}
