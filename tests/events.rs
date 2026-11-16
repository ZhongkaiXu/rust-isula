use risula::grpc::proto::{self, ExecRequest};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, UnixListener};
use tokio_stream::wrappers::{TcpListenerStream, UnixListenerStream};
use tonic::{Request, Response, Status};
struct FakeEvents {
    calls: Arc<Mutex<Vec<proto::EventsRequest>>>,
}
#[tonic::async_trait]
impl proto::container_service_server::ContainerService for FakeEvents {
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
    type EventsStream = tokio_stream::wrappers::ReceiverStream<Result<proto::Event, Status>>;
    async fn events(
        &self,
        request: Request<proto::EventsRequest>,
    ) -> Result<Response<Self::EventsStream>, Status> {
        let request = request.into_inner();
        self.calls.lock().unwrap().push(request.clone());
        if request.id == "missing" {
            return Err(Status::not_found("missing container"));
        }
        let (tx, rx) = tokio::sync::mpsc::channel(2);
        tokio::spawn(async move {
            if request.id == "empty" {
                return;
            }
            tx.send(Ok(proto::Event {
                timestamp: Some(prost_types::Timestamp {
                    seconds: 1700000000,
                    nanos: 123,
                }),
                opt: "container start".into(),
                id: "abc".into(),
                annotations: [("name".into(), "demo".into())].into(),
            }))
            .await
            .unwrap();
            if request.id == "slow" {
                tokio::time::sleep(std::time::Duration::from_secs(6)).await;
            }
            if request.id == "error" {
                let _ = tx.send(Err(Status::unknown("stream failed"))).await;
            } else {
                let _ = tx
                    .send(Ok(proto::Event {
                        opt: "container stop".into(),
                        id: "abc".into(),
                        ..Default::default()
                    }))
                    .await;
            }
        });
        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(
            rx,
        )))
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

async fn run_at(args: &[&str], tcp: bool) -> (Output, Vec<proto::EventsRequest>) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("isulad.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let tcp_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = tcp_listener.local_addr().unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let server_calls = calls.clone();
    let server = tokio::spawn(async move {
        let service = proto::container_service_server::ContainerServiceServer::new(FakeEvents {
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
        command.args(["events", "-H", &host]).args(&args);
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
async fn events_streams_over_unix_and_tcp() {
    for tcp in [false, true] {
        let (out, calls) = run_at(
            &[
                "-n",
                "demo",
                "-S",
                "2024-01-02T03:04:05.123456789",
                "-U",
                "2025-01-01",
                "ignored",
            ],
            tcp,
        )
        .await;
        assert!(out.status.success(), "{:?}", out);
        assert_eq!(out.stdout, b"2023-11-14T22:13:20.000000123+00:00 container start abc (name=demo)\n- container stop abc\n");
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "demo");
        assert!(!calls[0].store_only);
        assert_eq!(calls[0].since.as_ref().unwrap().seconds, 1704164645);
        assert_eq!(calls[0].since.as_ref().unwrap().nanos, 123456789);
        assert_eq!(calls[0].until.as_ref().unwrap().seconds, 1735689600);
    }
}
#[tokio::test]
async fn events_defaults_empty_stream_and_errors() {
    let (out, calls) = run_at(&["--name=", "--debug=false"], false).await;
    assert!(out.status.success());
    assert_eq!(calls[0], proto::EventsRequest::default());
    let (out, _) = run_at(&["-n", "empty"], false).await;
    assert!(out.status.success());
    assert!(out.stdout.is_empty());
    for name in ["missing", "error"] {
        let (out, _) = run_at(&["-n", name], false).await;
        assert_eq!(out.status.code(), Some(1));
        assert!(!out.stderr.is_empty());
        assert_eq!(out.stdout.is_empty(), name == "missing");
    }
}
#[tokio::test]
async fn events_validates_time_and_last_option_wins() {
    for args in [vec!["-S", "1h"], vec!["--since="], vec!["-U", "2024-02-30"]] {
        let (out, calls) = run_at(&args, false).await;
        assert_eq!(out.status.code(), Some(1));
        assert!(calls.is_empty());
    }
    let (out, calls) = run_at(
        &[
            "-S",
            "bad",
            "-S",
            "2024-01-01",
            "-n",
            "first",
            "-n",
            "empty",
        ],
        false,
    )
    .await;
    assert!(out.status.success());
    assert_eq!(calls[0].id, "empty");
}
#[tokio::test]
async fn events_subscription_has_no_five_second_deadline() {
    let (out, _) = run_at(&["-n", "slow"], true).await;
    assert!(out.status.success(), "{:?}", out);
    assert!(out.stdout.ends_with(b"- container stop abc\n"));
}
