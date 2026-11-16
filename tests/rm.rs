use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

use risula::grpc::proto::{self, DeleteRequest, DeleteResponse};
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{Request, Response, Status};

struct FakeRemove {
    calls: Arc<Mutex<Vec<DeleteRequest>>>,
    id: String,
}

#[tonic::async_trait]
impl proto::container_service_server::ContainerService for FakeRemove {
    async fn update(
        &self,
        _: Request<proto::UpdateRequest>,
    ) -> Result<Response<proto::UpdateResponse>, Status> {
        Err(Status::unimplemented("update"))
    }
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
        _request: Request<proto::StopRequest>,
    ) -> Result<Response<proto::StopResponse>, Status> {
        Err(Status::unimplemented("stop"))
    }

    async fn delete(
        &self,
        request: Request<DeleteRequest>,
    ) -> Result<Response<DeleteResponse>, Status> {
        let request = request.into_inner();
        self.calls.lock().unwrap().push(request.clone());
        if request.id == "missing" {
            return Ok(Response::new(DeleteResponse {
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
        Ok(Response::new(DeleteResponse {
            id: if request.id.starts_with("no-id-") {
                String::new()
            } else if request.id == "unsafe-id" {
                "../outside".into()
            } else {
                self.id.clone()
            },
            // Upstream uses cc, not exit_status or a nonempty success errmsg.
            exit_status: 137,
            errmsg: "ignored on success".into(),
            ..Default::default()
        }))
    }
}

async fn run_rm(args: &[&str]) -> (Output, Vec<DeleteRequest>, String) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("isulad.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let server_calls = calls.clone();
    let id = format!(
        "risula-rm-test-{}",
        directory.path().file_name().unwrap().to_string_lossy()
    );
    let server_id = id.clone();
    let server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(
                proto::container_service_server::ContainerServiceServer::new(FakeRemove {
                    calls: server_calls,
                    id: server_id,
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
            .args(["rm", "-H", &host])
            .args(args)
            .output()
            .unwrap()
    })
    .await
    .unwrap();
    server.abort();
    let seen = calls.lock().unwrap().clone();
    (output, seen, id)
}

#[tokio::test]
async fn rm_defaults_and_prints_resolved_id_for_each_container() {
    let (output, calls, id) = run_rm(&["first", "second"]).await;
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        calls,
        vec![
            DeleteRequest {
                id: "first".into(),
                force: false,
                volumes: false
            },
            DeleteRequest {
                id: "second".into(),
                force: false,
                volumes: false
            },
        ]
    );
    assert_eq!(output.stdout, format!("{id}\n{id}\n").as_bytes());
    assert!(output.stderr.is_empty(), "{output:?}");
}

#[tokio::test]
async fn rm_sends_force_and_volumes_with_short_long_and_combined_flags() {
    for (args, force, volumes) in [
        (vec!["-f", "test"], true, false),
        (vec!["-v", "test"], false, true),
        (vec!["-fv", "test"], true, true),
        (vec!["-vf", "test"], true, true),
        (vec!["--force", "--volumes", "test"], true, true),
        (
            vec!["-fv", "--force=false", "--volumes=false", "test"],
            false,
            false,
        ),
        (vec!["-f=0", "-v=1", "test"], false, true),
        (
            vec!["--force=false", "-f", "--volumes=false", "-v", "test"],
            true,
            true,
        ),
    ] {
        let (output, calls, _) = run_rm(&args).await;
        assert!(output.status.success(), "{args:?}: {output:?}");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].force, force, "{args:?}");
        assert_eq!(calls[0].volumes, volumes, "{args:?}");
    }
}

#[tokio::test]
async fn rm_accepts_upstream_boolean_values() {
    for (values, expected) in [
        (["1", "t", "T", "true", "TRUE", "True"], true),
        (["0", "f", "F", "false", "FALSE", "False"], false),
    ] {
        for value in values {
            let (output, calls, _) = run_rm(&[
                &format!("--force={value}"),
                &format!("--volumes={value}"),
                "test",
            ])
            .await;
            assert!(output.status.success(), "{value}: {output:?}");
            assert_eq!(calls[0].force, expected);
            assert_eq!(calls[0].volumes, expected);
        }
    }
}

#[tokio::test]
async fn rm_rejects_invalid_options_before_any_delete() {
    for args in [
        vec!["--force=bad"],
        vec!["--volumes=bad", "--help"],
        vec!["-v=bad"],
        vec!["--force=", "test"],
        vec!["--volumes=", "test"],
        vec!["-f=no", "test"],
        vec!["-v=yes", "test"],
        vec!["--force=bad", "--force=true", "test"],
        vec!["--volumes=bad", "--volumes=false", "test"],
        vec!["-v==true", "test"],
        vec!["--unknown", "test"],
        vec!["-l", "test"],
    ] {
        let (output, calls, _) = run_rm(&args).await;
        assert_eq!(output.status.code(), Some(125), "{args:?}: {output:?}");
        assert!(calls.is_empty());
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
}

#[tokio::test]
async fn rm_checks_container_count_with_exit_one() {
    for args in [vec![], vec!["test"; 1000]] {
        let (output, calls, _) = run_rm(&args).await;
        assert_eq!(output.status.code(), Some(1));
        assert!(calls.is_empty());
    }
    let (output, calls, _) = run_rm(&vec!["test"; 999]).await;
    assert!(output.status.success());
    assert_eq!(calls.len(), 999);
}

#[tokio::test]
async fn rm_continues_after_business_transport_and_empty_name_errors() {
    let (output, calls, id) = run_rm(&["missing", "", "unavailable", "good"]).await;
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        calls.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
        ["missing", "unavailable", "good"]
    );
    assert_eq!(output.stdout, format!("{id}\n").as_bytes());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("code 42"));
    assert!(stderr.contains("No such container:missing"));
    assert!(stderr.contains("container name cannot be empty"));
    assert!(stderr.contains("container service unavailable"));
}

#[tokio::test]
async fn rm_uses_original_name_when_response_id_is_empty() {
    let name = format!("no-id-risula-test-{}", std::process::id());
    let (output, _, _) = run_rm(&[&name]).await;
    assert!(output.status.success());
    assert_eq!(output.stdout, format!("{name}\n").as_bytes());
    assert!(output.stderr.is_empty());
}

#[tokio::test]
async fn rm_cleanup_failure_does_not_change_success_or_skip_next_container() {
    let (output, calls, id) = run_rm(&["unsafe-id", "good"]).await;
    assert!(output.status.success());
    assert_eq!(calls.len(), 2);
    assert_eq!(output.stdout, format!("../outside\n{id}\n").as_bytes());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("cannot clean console directory"));
}

#[tokio::test]
async fn rm_waits_for_slow_removal() {
    let (output, calls, _) = run_rm(&["slow"]).await;
    assert!(output.status.success(), "{output:?}");
    assert_eq!(calls.len(), 1);
}

#[tokio::test]
async fn rm_help_and_option_terminator() {
    let (output, calls, _) = run_rm(&["--help"]).await;
    assert!(output.status.success());
    assert!(calls.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--force"));
    assert!(stdout.contains("--volumes"));
    let (output, calls, _) = run_rm(&["--", "-name"]).await;
    assert!(output.status.success());
    assert_eq!(calls[0].id, "-name");
}

#[test]
fn rm_reports_connection_failure_for_each_container() {
    let directory = tempfile::tempdir().unwrap();
    let host = format!("unix://{}/missing.sock", directory.path().display());
    let output = Command::new(env!("CARGO_BIN_EXE_risula"))
        .env("ISULAD_HOST", host)
        .args(["rm", "first", "second"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(stderr.matches("cannot connect").count(), 2);
    assert!(stderr.contains("first"));
    assert!(stderr.contains("second"));
}
