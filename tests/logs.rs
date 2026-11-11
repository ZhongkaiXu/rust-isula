use risula::grpc::proto::{self, ExecRequest};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, UnixListener};
use tokio_stream::wrappers::{TcpListenerStream, UnixListenerStream};
use tonic::{Request, Response, Status};
struct FakeLogs {
    calls: Arc<Mutex<Vec<proto::LogsRequest>>>,
}
#[tonic::async_trait]
impl proto::container_service_server::ContainerService for FakeLogs {
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
    type LogsStream = tokio_stream::wrappers::ReceiverStream<Result<proto::LogsResponse, Status>>;
    async fn logs(
        &self,
        request: Request<proto::LogsRequest>,
    ) -> Result<Response<Self::LogsStream>, Status> {
        let request = request.into_inner();
        self.calls.lock().unwrap().push(request.clone());
        if request.id == "disabled" {
            return Err(Status::unknown("disable console log for container"));
        }
        if request.id == "missing" {
            return Err(Status::not_found("missing container"));
        }
        let (tx, rx) = tokio::sync::mpsc::channel(2);
        tokio::spawn(async move {
            if request.id == "empty" {
                return;
            }
            if request.id == "invalid" {
                tx.send(Ok(proto::LogsResponse {
                    stream: "unknown".into(),
                    data: b"discard".to_vec(),
                    ..Default::default()
                }))
                .await
                .unwrap();
            }
            tx.send(Ok(proto::LogsResponse {
                stream: "stdout".into(),
                data: b"out\0\xff\n".to_vec(),
                time: "2026-11-11T01:02:03.123456789Z".into(),
                ..Default::default()
            }))
            .await
            .unwrap();
            if request.follow {
                tokio::time::sleep(std::time::Duration::from_secs(6)).await;
            }
            if request.id == "stream-error" || request.id == "late-disabled" {
                let _ = tx
                    .send(Err(Status::unknown(if request.id == "late-disabled" {
                        "disable console log"
                    } else {
                        "stream failed"
                    })))
                    .await;
            } else {
                let _ = tx
                    .send(Ok(proto::LogsResponse {
                        stream: "stderr".into(),
                        data: b"err".to_vec(),
                        time: "2026-11-11T01:02:04Z".into(),
                        ..Default::default()
                    }))
                    .await;
            }
        });
        Ok(Response::new(tokio_stream::wrappers::ReceiverStream::new(
            rx,
        )))
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

async fn run_at(args: &[&str], tcp: bool) -> (Output, Vec<proto::LogsRequest>) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("isulad.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let tcp_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = tcp_listener.local_addr().unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let server_calls = calls.clone();
    let server = tokio::spawn(async move {
        let service = proto::container_service_server::ContainerServiceServer::new(FakeLogs {
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
        command.args(["logs", "-H", &host]).args(&args);
        if args.iter().any(|arg| arg == "-f") {
            use std::io::Read;
            let start = std::time::Instant::now();
            let mut child = command
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .unwrap();
            let mut first = [0; 6];
            child
                .stdout
                .as_mut()
                .unwrap()
                .read_exact(&mut first)
                .unwrap();
            assert_eq!(&first, b"out\0\xff\n");
            assert!(start.elapsed() < std::time::Duration::from_secs(3));
            let mut out = child.wait_with_output().unwrap();
            out.stdout.splice(0..0, first);
            out
        } else {
            command.output().unwrap()
        }
    })
    .await
    .unwrap();
    server.abort();
    let seen = calls.lock().unwrap().clone();
    (output, seen)
}

#[tokio::test]
async fn logs_preserves_bytes_streams_and_defaults() {
    for tcp in [false, true] {
        let (out, calls) = run_at(&["demo"], tcp).await;
        assert!(out.status.success());
        assert_eq!(out.stdout, b"out\0\xff\n");
        assert_eq!(out.stderr, b"err");
        assert_eq!(
            calls,
            vec![proto::LogsRequest {
                id: "demo".into(),
                tail: -1,
                ..Default::default()
            }]
        );
    }
}

#[tokio::test]
async fn logs_formats_timestamps_without_adding_newlines() {
    let (out, calls) = run_at(&["-t", "--tail=010", "demo"], false).await;
    assert!(out.status.success());
    assert_eq!(calls[0].tail, 8);
    assert!(calls[0].timestamps);
    assert_eq!(out.stdout, b"2026-11-11T01:02:03.123456789Z out\0\xff\n");
    assert_eq!(out.stderr, b"2026-11-11T01:02:04Z err");
}

#[tokio::test]
async fn logs_boolean_overrides_and_invalid_tail_fallback() {
    for (text, tail) in [
        ("all", -1),
        ("", -1),
        ("nonsense", -1),
        ("0x10", 16),
        ("-2", -2),
        ("9223372036854775808", -1),
        ("0", 0),
    ] {
        let (out, calls) = run_at(
            &[
                "-ft",
                "--follow=false",
                "--timestamps=false",
                "--tail",
                text,
                "demo",
            ],
            false,
        )
        .await;
        assert!(out.status.success());
        assert_eq!(calls[0].tail, tail);
        assert!(!calls[0].follow && !calls[0].timestamps);
    }
}

#[tokio::test]
async fn logs_disabled_is_a_successful_warning_and_errors_preserve_prior_output() {
    for name in [
        "disabled",
        "late-disabled",
        "missing",
        "stream-error",
        "empty",
    ] {
        let (out, _) = run_at(&[name], true).await;
        if name.contains("disabled") {
            assert!(out.status.success());
            assert!(String::from_utf8_lossy(&out.stdout).contains(&format!(
                "[WARNING]: Container {name} disable console log!\n"
            )));
            assert!(out.stderr.is_empty());
        } else if name == "empty" {
            assert!(out.status.success());
            assert!(out.stdout.is_empty() && out.stderr.is_empty());
        } else {
            assert_eq!(out.status.code(), Some(1));
            assert!(!out.stderr.is_empty());
        }
        if name == "stream-error" {
            assert_eq!(out.stdout, b"out\0\xff\n");
        }
    }
}

#[tokio::test]
async fn logs_reports_unknown_stream_but_continues() {
    let (out, _) = run_at(&["invalid"], false).await;
    assert!(out.status.success());
    assert_eq!(out.stdout, b"out\0\xff\n");
    assert!(String::from_utf8_lossy(&out.stderr).contains("Invalid container log: unknown"));
}

#[tokio::test]
async fn logs_validates_cli_before_rpc() {
    for args in [
        vec![],
        vec!["a", "b"],
        vec!["--since", "today", "demo"],
        vec!["--follow=bad", "demo"],
        vec!["--tail"],
    ] {
        let (out, calls) = run_at(&args, false).await;
        assert!(!out.status.success());
        assert!(calls.is_empty());
    }
    let (out, calls) = run_at(&["--help"], false).await;
    assert!(out.status.success());
    assert!(calls.is_empty());
}

#[tokio::test]
async fn logs_follow_flushes_immediately_and_has_no_five_second_deadline() {
    for tcp in [false, true] {
        let (out, calls) = run_at(&["-f", "demo"], tcp).await;
        assert!(out.status.success());
        assert_eq!(out.stdout, b"out\0\xff\n");
        assert_eq!(out.stderr, b"err");
        assert!(calls[0].follow);
    }
}
