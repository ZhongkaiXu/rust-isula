use risula::grpc::proto::{self, ExecRequest};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, UnixListener};
use tokio_stream::wrappers::{TcpListenerStream, UnixListenerStream};
use tonic::{Request, Response, Status};
struct FakePort {
    calls: Arc<Mutex<Vec<proto::InspectContainerRequest>>>,
}
#[tonic::async_trait]
impl proto::container_service_server::ContainerService for FakePort {
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
        request: Request<proto::InspectContainerRequest>,
    ) -> Result<Response<proto::InspectContainerResponse>, Status> {
        let request = request.into_inner();
        self.calls.lock().unwrap().push(request.clone());
        if request.id == "rpc-error" {
            return Err(Status::unavailable("failed"));
        }
        let json = match request.id.as_str() {
            "empty" => "{}",
            "null" => r#"{"NetworkSettings":{"Ports":null}}"#,
            "invalid" => "bad json",
            "wrong-type" => r#"{"NetworkSettings":{"Ports":[]}}"#,
            _ => {
                r#"{"NetworkSettings":{"Ports":{"80/tcp":{"Element":{"host":[{"HostIP":"127.0.0.1","HostPort":"8080"},{"HostIP":"","HostPort":"8081"}]}},"53/udp":{"Element":{"host":[{"HostIP":"::1","HostPort":"5353"}]}},"81/tcp":null,"82/tcp":{},"83/tcp":{"Element":{"host":[]}}}}}"#
            }
        };
        Ok(Response::new(proto::InspectContainerResponse {
            container_json: json.into(),
            cc: if request.id == "missing" { 2 } else { 0 },
            errmsg: "No such image or container or accelerator".into(),
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
    async fn top(
        &self,
        _: Request<proto::TopRequest>,
    ) -> Result<Response<proto::TopResponse>, Status> {
        Err(Status::unimplemented("top"))
    }
}

async fn run_at(args: &[&str], tcp: bool) -> (Output, Vec<proto::InspectContainerRequest>) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("isulad.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let tcp_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = tcp_listener.local_addr().unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let server_calls = calls.clone();
    let server = tokio::spawn(async move {
        let service = proto::container_service_server::ContainerServiceServer::new(FakePort {
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
        command.args(["port", "-H", &host]).args(&args);
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
async fn port_lists_bindings_and_preserves_order_over_both_transports() {
    for tcp in [false, true] {
        let (out, calls) = run_at(&["demo"], tcp).await;
        assert!(out.status.success(), "{out:?}");
        assert_eq!(
            out.stdout,
            b"80/tcp -> 127.0.0.1:8080\n80/tcp -> 0.0.0.0:8081\n53/udp -> ::1:5353\n"
        );
        assert!(out.stderr.is_empty());
        assert_eq!(
            calls,
            vec![proto::InspectContainerRequest {
                id: "demo".into(),
                timeout: 120,
                bformat: false
            }]
        );
    }
}
#[tokio::test]
async fn port_filters_normalized_keys_and_ignores_extra_arguments() {
    for input in ["80", "0x50/tcp", "0120", "80-80", "80//tcp"] {
        let (out, _) = run_at(&["--debug=false", "demo", input, "ignored"], false).await;
        assert!(out.status.success());
        assert_eq!(
            out.stdout,
            b"80/tcp -> 127.0.0.1:8080\n80/tcp -> 0.0.0.0:8081\n"
        );
    }
    for input in ["80/UDP", "80/unknown", "80-81", "65535"] {
        let (out, _) = run_at(&["demo", input], false).await;
        assert!(out.status.success());
        assert!(out.stdout.is_empty());
    }
}
#[tokio::test]
async fn port_empty_mappings_succeed_and_inspect_errors_fail() {
    for name in ["empty", "null"] {
        let (out, _) = run_at(&[name], false).await;
        assert!(out.status.success());
        assert!(out.stdout.is_empty());
    }
    for name in ["missing", "rpc-error", "invalid", "wrong-type"] {
        let (out, calls) = run_at(&[name], false).await;
        assert_eq!(out.status.code(), Some(255));
        assert!(out.stdout.is_empty());
        assert_eq!(calls.len(), 1);
    }
}
#[tokio::test]
async fn port_invalid_arguments_never_inspect() {
    for (args, code) in [
        (vec![], 125),
        (vec!["--unknown"], 125),
        (vec![""], 255),
        (vec!["demo", "08"], 255),
        (vec!["demo", "0"], 255),
        (vec!["demo", "65536"], 255),
        (vec!["demo", "80/"], 255),
        (vec!["demo"; 1000], 125),
    ] {
        let (out, calls) = run_at(&args, false).await;
        assert_eq!(out.status.code(), Some(code), "{args:?}: {out:?}");
        assert!(calls.is_empty());
    }
}
