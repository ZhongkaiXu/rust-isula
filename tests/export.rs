use risula::grpc::proto::{self, ExecRequest};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, UnixListener};
use tokio_stream::wrappers::{TcpListenerStream, UnixListenerStream};
use tonic::{Request, Response, Status};
struct FakeExport {
    calls: Arc<Mutex<Vec<proto::ExportRequest>>>,
}
#[tonic::async_trait]
impl proto::container_service_server::ContainerService for FakeExport {
    async fn update(
        &self,
        _: Request<proto::UpdateRequest>,
    ) -> Result<Response<proto::UpdateResponse>, Status> {
        Err(Status::unimplemented("update"))
    }
    async fn export(
        &self,
        request: Request<proto::ExportRequest>,
    ) -> Result<Response<proto::ExportResponse>, Status> {
        let request = request.into_inner();
        self.calls.lock().unwrap().push(request.clone());
        if request.id == "rpc-error" {
            return Err(Status::unavailable("transport failed"));
        }
        if request.id == "slow" {
            tokio::time::sleep(std::time::Duration::from_secs(6)).await;
        }
        Ok(Response::new(proto::ExportResponse {
            id: request.id.clone(),
            cc: if request.id == "failure" { 2 } else { 0 },
            errmsg: "server message".into(),
        }))
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
        _request: Request<ExecRequest>,
    ) -> Result<Response<proto::ExecResponse>, Status> {
        Err(Status::unimplemented("exec"))
    }
    type RemoteExecStream =
        tokio_stream::Iter<std::vec::IntoIter<Result<proto::RemoteExecResponse, Status>>>;
    async fn remote_exec(
        &self,
        _request: Request<tonic::Streaming<proto::RemoteExecRequest>>,
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
    async fn wait(
        &self,
        _: Request<proto::WaitRequest>,
    ) -> Result<Response<proto::WaitResponse>, Status> {
        Err(Status::unimplemented("wait"))
    }
    async fn top(
        &self,
        _: Request<proto::TopRequest>,
    ) -> Result<Response<proto::TopResponse>, Status> {
        Err(Status::unimplemented("top"))
    }
}

async fn run_at(args: &[&str], tcp: bool) -> (Output, Vec<proto::ExportRequest>) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("isulad.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let tcp_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = tcp_listener.local_addr().unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let server_calls = calls.clone();
    let server = tokio::spawn(async move {
        let service = proto::container_service_server::ContainerServiceServer::new(FakeExport {
            calls: server_calls,
        });
        let server = tonic::transport::Server::builder().add_service(service);
        if tcp {
            server
                .serve_with_incoming(TcpListenerStream::new(tcp_listener))
                .await
                .unwrap();
        } else {
            server
                .serve_with_incoming(UnixListenerStream::new(listener))
                .await
                .unwrap();
        }
    });
    let host = if tcp {
        format!("tcp://{address}")
    } else {
        format!("unix://{}", socket.display())
    };
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let output = tokio::task::spawn_blocking(move || {
        let mut command = Command::new(env!("CARGO_BIN_EXE_risula"));
        command.args(["export", "-H", &host]).args(&args);
        command.env("TZ", "UTC");
        command.output().unwrap()
    })
    .await
    .unwrap();
    server.abort();
    let seen = calls.lock().unwrap().clone();
    (output, seen)
}

#[tokio::test]
async fn export_sends_absolute_path_over_unix_and_tcp_and_is_silent() {
    for tcp in [false, true] {
        let (out, calls) = run_at(&["-o", "/daemon/only/archive.tar", "demo"], tcp).await;
        assert!(out.status.success(), "{:?}", out);
        assert!(out.stdout.is_empty() && out.stderr.is_empty());
        assert_eq!(
            calls,
            vec![proto::ExportRequest {
                id: "demo".into(),
                file: "/daemon/only/archive.tar".into()
            }]
        );
    }
}
#[tokio::test]
async fn export_preserves_relative_components_empty_output_and_last_option() {
    for value in ["./missing/../out.tar", "", "-"] {
        let (out, calls) = run_at(
            &[
                "--debug=false",
                "-o",
                "first",
                &format!("--output={value}"),
                "demo",
            ],
            false,
        )
        .await;
        assert!(out.status.success());
        assert_eq!(
            calls[0].file,
            format!("{}/{value}", std::env::current_dir().unwrap().display())
        );
    }
}
#[tokio::test]
async fn export_rejects_invalid_arguments_without_rpc() {
    let long = "a".repeat(libc::PATH_MAX as usize);
    for (args, code) in [
        (vec![], 125),
        (vec!["demo"], 125),
        (vec!["-o", "x"], 125),
        (vec!["-o", "x", "a", "b"], 125),
        (vec!["-o"], 125),
        (vec!["--unknown"], 125),
        (vec!["-o", "x", ""], 1),
        (vec!["-o", &long, "demo"], 125),
    ] {
        let (out, calls) = run_at(&args, false).await;
        assert_eq!(out.status.code(), Some(code), "{args:?}: {out:?}");
        assert!(calls.is_empty());
    }
}
#[tokio::test]
async fn export_propagates_server_and_transport_failures() {
    for name in ["failure", "rpc-error"] {
        let (out, calls) = run_at(&["-o", "x", name], false).await;
        assert_eq!(out.status.code(), Some(1));
        assert_eq!(calls.len(), 1);
        assert!(out.stdout.is_empty());
        assert!(!out.stderr.is_empty());
    }
}
#[tokio::test]
async fn export_waits_for_archive_without_five_second_deadline() {
    let (out, _) = run_at(&["-o", "x", "slow"], true).await;
    assert!(out.status.success(), "{out:?}");
}
