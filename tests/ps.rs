use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

use risula::grpc::proto::{self, ListRequest, ListResponse};
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{Request, Response, Status};

struct FakeList {
    calls: Arc<Mutex<Vec<ListRequest>>>,
    reply: ListResponse,
}

#[tonic::async_trait]
impl proto::container_service_server::ContainerService for FakeList {
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

    async fn list(&self, request: Request<ListRequest>) -> Result<Response<ListResponse>, Status> {
        self.calls.lock().unwrap().push(request.into_inner());
        Ok(Response::new(self.reply.clone()))
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

    async fn stop(
        &self,
        _: Request<proto::StopRequest>,
    ) -> Result<Response<proto::StopResponse>, Status> {
        Err(Status::unimplemented("stop"))
    }
}

async fn run_ps(reply: ListResponse, args: &[&str]) -> (Output, Vec<ListRequest>) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("isulad.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let server_calls = calls.clone();
    let server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(
                proto::container_service_server::ContainerServiceServer::new(FakeList {
                    calls: server_calls,
                    reply,
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
            .args(["ps", "-H", &host])
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

fn sample() -> ListResponse {
    ListResponse {
        containers: vec![
            proto::Container {
                id: "a".repeat(64),
                name: "older".into(),
                image: "busybox:latest".into(),
                command: "sleep 300".into(),
                status: 4,
                exit_code: 7,
                startat: "2026-01-01T00:00:00Z".into(),
                finishat: "2026-01-01T00:05:00Z".into(),
                runtime: "runc".into(),
                ..Default::default()
            },
            proto::Container {
                id: "b".repeat(64),
                name: "newer".into(),
                image: "busybox:latest".into(),
                command: "sh -c 'a very long command string'".into(),
                status: 3,
                pid: 123,
                health_state: "healthy".into(),
                startat: "2026-02-01T00:00:00Z".into(),
                runtime: "runc".into(),
                ..Default::default()
            },
        ],
        ..Default::default()
    }
}

#[tokio::test]
async fn ps_filters_and_quiet_full_ids() {
    let (out, calls) = run_ps(sample(), &["-aq", "-f", "status=exited"]).await;
    assert!(out.status.success());
    assert!(calls[0].all);
    assert_eq!(calls[0].filters["status"], "exited");
    assert_eq!(
        String::from_utf8(out.stdout).unwrap(),
        format!("{}\n{}\n", "b".repeat(64), "a".repeat(64))
    );
}

#[tokio::test]
async fn ps_default_request_and_table() {
    let (out, calls) = run_ps(sample(), &[]).await;
    assert!(out.status.success());
    assert_eq!(calls, vec![ListRequest::default()]);
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.starts_with("CONTAINER ID\tIMAGE"));
    assert!(text.contains("aaaaaaaaaaaa\t"));
    assert!(!text.contains(&"a".repeat(64)));
    assert!(text.contains("Exited (7)"));
    assert!(text.contains("(healthy)"));
}

#[tokio::test]
async fn ps_latest_overrides_last_and_filters() {
    let (_, calls) = run_ps(sample(), &["-n", "3", "-l", "-f", "name=older"]).await;
    assert_eq!(
        calls[0].filters,
        std::collections::HashMap::from([("last_n".into(), "1".into())])
    );
    let (_, calls) = run_ps(sample(), &["-n", "2"]).await;
    assert_eq!(calls[0].filters["last_n"], "2");
}

#[tokio::test]
async fn ps_custom_fields_and_no_trunc() {
    let (out, _) = run_ps(
        sample(),
        &[
            "--no-trunc",
            "--format",
            "{{.ID}}|{{.State}}|{{.Pid}}|{{.ExitCode}}|{{.Runtime}}",
        ],
    )
    .await;
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.starts_with(&"a".repeat(64)));
    assert!(text.contains("exited"));
    assert!(text.contains("123"));
    assert!(!text.contains("CONTAINER ID"));
}

#[tokio::test]
async fn ps_empty_table_and_daemon_error() {
    let (out, _) = run_ps(ListResponse::default(), &[]).await;
    assert!(out.status.success());
    assert_eq!(String::from_utf8(out.stdout).unwrap().lines().count(), 1);
    let (out, _) = run_ps(
        ListResponse {
            cc: 42,
            errmsg: "invalid filter".into(),
            ..Default::default()
        },
        &[],
    )
    .await;
    assert!(!out.status.success());
    let text = String::from_utf8(out.stderr).unwrap();
    assert!(text.contains("42") && text.contains("invalid filter"));
}

#[tokio::test]
async fn ps_invalid_arguments_do_not_call_daemon() {
    for args in [
        vec!["-n", "-1"],
        vec!["-n", "bad"],
        vec!["-f", "broken"],
        vec!["--format", "{{.Unknown}}"],
        vec!["--format", "{{.ID"],
        vec!["extra"],
    ] {
        let (out, calls) = run_ps(sample(), &args).await;
        assert!(!out.status.success(), "{args:?}");
        assert!(calls.is_empty());
    }
}

#[tokio::test]
async fn ps_custom_command_preserves_upstream_width() {
    let (out, _) = run_ps(sample(), &["--format", "{{.Command}}|end"]).await;
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    let lines: Vec<_> = text.lines().collect();
    assert_eq!(lines[0], format!("{:<36}|end", "\"sleep 300\""));
    assert_eq!(lines[1], "\"sh -c 'a very lon...\" |end");
}
