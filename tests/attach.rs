use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

use risula::grpc::proto::{self, ExecRequest};
use tokio::net::{TcpListener, UnixListener};
use tokio_stream::wrappers::{TcpListenerStream, UnixListenerStream};
use tonic::{Request, Response, Status};

#[derive(Default, Clone)]
struct Calls {
    inspected: Vec<String>,
    attached: Vec<String>,
    waited: Vec<String>,
    input: Vec<u8>,
}
struct FakeAttach {
    calls: Arc<Mutex<Calls>>,
}

#[tonic::async_trait]
impl proto::container_service_server::ContainerService for FakeAttach {
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

    type AttachStream =
        tokio_stream::wrappers::ReceiverStream<Result<proto::AttachResponse, Status>>;
    async fn attach(
        &self,
        request: Request<tonic::Streaming<proto::AttachRequest>>,
    ) -> Result<Response<Self::AttachStream>, Status> {
        for key in ["attach-stdin", "attach-stdout", "attach-stderr"] {
            assert_eq!(request.metadata().get(key).unwrap(), "true");
        }
        let id = request
            .metadata()
            .get("container-id")
            .unwrap()
            .to_str()
            .unwrap()
            .to_owned();
        self.calls.lock().unwrap().attached.push(id.clone());
        let (tx, rx) = tokio::sync::mpsc::channel(2);
        let mut response = Response::new(tokio_stream::wrappers::ReceiverStream::new(rx));
        response.metadata_mut().insert(
            "cc",
            if id == "full-error" {
                "1"
            } else if id == "full-bad-status" {
                "bad"
            } else {
                "0"
            }
            .parse()
            .unwrap(),
        );
        response
            .metadata_mut()
            .insert("errmsg", "attach denied".parse().unwrap());
        let calls = self.calls.clone();
        tokio::spawn(async move {
            let mut input = request.into_inner();
            if id == "full-echo" {
                while let Some(frame) = input.message().await.unwrap() {
                    if frame.finish {
                        break;
                    }
                    calls.lock().unwrap().input.extend_from_slice(&frame.stdin);
                }
            }
            if id == "full-slow" {
                tokio::time::sleep(std::time::Duration::from_secs(6)).await;
            }
            if id == "full-stream-error" {
                let _ = tx.send(Err(Status::internal("stream failed"))).await;
                return;
            }
            let output = if id == "full-echo" {
                calls.lock().unwrap().input.clone()
            } else {
                b"out\0\xff".to_vec()
            };
            if tx
                .send(Ok(proto::AttachResponse {
                    stdout: output,
                    stderr: b"err".to_vec(),
                    finish: false,
                }))
                .await
                .is_err()
            {
                return;
            }
            if tx
                .send(Ok(proto::AttachResponse {
                    finish: true,
                    ..Default::default()
                }))
                .await
                .is_err()
            {
                return;
            }
            // Match the daemon: trailers cannot complete until the client closes stdin.
            while let Ok(Some(_)) = input.message().await {}
        });
        Ok(response)
    }

    async fn exec(&self, _: Request<ExecRequest>) -> Result<Response<proto::ExecResponse>, Status> {
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
        assert!(!request.bformat);
        assert_eq!(request.timeout, 120);
        let name = request.id;
        self.calls.lock().unwrap().inspected.push(name.clone());
        if name == "missing" {
            return Ok(Response::new(proto::InspectContainerResponse {
                cc: 1,
                errmsg: "no such container".into(),
                ..Default::default()
            }));
        }
        let json = serde_json::json!({"Id": format!("full-{name}"), "State": {"Running": name != "stopped", "Paused": name == "paused", "Restarting": name == "restarting"}, "Config": {"Tty": name == "tty"}});
        Ok(Response::new(proto::InspectContainerResponse {
            container_json: if name == "bad-json" {
                "{".into()
            } else {
                json.to_string()
            },
            ..Default::default()
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
        request: Request<proto::WaitRequest>,
    ) -> Result<Response<proto::WaitResponse>, Status> {
        let request = request.into_inner();
        assert_eq!(request.condition, 0);
        self.calls.lock().unwrap().waited.push(request.id.clone());
        if request.id == "full-detach" {
            return std::future::pending().await;
        }
        if request.id == "full-wait-error" {
            return Err(Status::internal("wait failed"));
        }
        if request.id == "full-slow" {
            tokio::time::sleep(std::time::Duration::from_secs(6)).await;
        }
        let code = if request.id == "full-demo" || request.id == "full-slow" {
            7
        } else {
            0
        };
        Ok(Response::new(proto::WaitResponse {
            exit_code: code,
            ..Default::default()
        }))
    }
    async fn top(
        &self,
        _: Request<proto::TopRequest>,
    ) -> Result<Response<proto::TopResponse>, Status> {
        Err(Status::unimplemented("top"))
    }
}

async fn run_at(args: &[&str], tcp: bool, data: Option<Vec<u8>>) -> (Output, Calls) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("isulad.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let tcp_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = tcp_listener.local_addr().unwrap();
    let calls = Arc::new(Mutex::new(Calls::default()));
    let server_calls = calls.clone();
    let server = tokio::spawn(async move {
        let service = proto::container_service_server::ContainerServiceServer::new(FakeAttach {
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
        use std::io::Write;
        use std::process::Stdio;
        let mut child = Command::new(env!("CARGO_BIN_EXE_risula"))
            .args(["attach", "-H", &host])
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        let writer = std::thread::spawn(move || {
            if let Some(data) = data {
                stdin.write_all(&data).unwrap();
            } else {
                std::thread::sleep(std::time::Duration::from_millis(300));
            }
        });
        let output = child.wait_with_output().unwrap();
        writer.join().unwrap();
        output
    })
    .await
    .unwrap();
    server.abort();
    let seen = calls.lock().unwrap().clone();
    (output, seen)
}

#[tokio::test]
async fn attach_streams_binary_output_and_uses_wait_exit_status_on_both_transports() {
    for tcp in [false, true] {
        let (out, calls) = run_at(&["demo"], tcp, None).await;
        assert_eq!(out.status.code(), Some(7));
        assert_eq!(out.stdout, b"out\0\xff");
        assert_eq!(out.stderr, b"err");
        assert_eq!(calls.attached, ["full-demo"]);
        assert_eq!(calls.waited, ["full-demo"]);
    }
}

#[tokio::test]
async fn attach_streams_large_input_and_keeps_output_after_stdin_eof() {
    let data: Vec<u8> = (0..131072).map(|n| n as u8).collect();
    for tcp in [false, true] {
        let (out, calls) = run_at(&["echo"], tcp, Some(data.clone())).await;
        assert!(out.status.success(), "{:?}", out.stderr);
        assert_eq!(out.stdout, data);
        assert_eq!(calls.input, data);
    }
}

#[tokio::test]
async fn attach_checks_state_and_tty_before_stream_or_wait() {
    for name in [
        "missing",
        "stopped",
        "paused",
        "restarting",
        "bad-json",
        "tty",
    ] {
        let (out, calls) = run_at(&[name], false, None).await;
        assert_eq!(out.status.code(), Some(1), "{name}");
        assert!(calls.attached.is_empty() && calls.waited.is_empty());
        assert!(!out.stderr.is_empty());
    }
}

#[tokio::test]
async fn attach_validates_cli_before_inspection() {
    for args in [vec![], vec!["a", "b"], vec![""], vec!["--no-stdin", "demo"]] {
        let (out, calls) = run_at(&args, false, None).await;
        assert!(!out.status.success());
        assert!(calls.inspected.is_empty());
    }
    let (out, calls) = run_at(&["--help"], false, None).await;
    assert!(out.status.success());
    assert!(calls.inspected.is_empty());
}

#[tokio::test]
async fn attach_reports_headers_and_stream_errors() {
    for name in ["error", "bad-status", "stream-error"] {
        let (out, _) = run_at(&[name], true, None).await;
        assert_eq!(out.status.code(), Some(1));
        assert!(
            String::from_utf8_lossy(&out.stderr).contains(if name == "error" {
                "attach denied"
            } else if name == "bad-status" {
                "invalid attach status"
            } else {
                "stream failed"
            })
        );
    }
}

#[tokio::test]
async fn attach_limits_post_stream_wait_and_handles_wait_failure() {
    for name in ["detach", "wait-error"] {
        let start = std::time::Instant::now();
        let (out, _) = run_at(&[name], false, None).await;
        assert_eq!(out.status.code(), Some(1));
        assert!(start.elapsed() < std::time::Duration::from_secs(4));
    }
}

#[tokio::test]
async fn attach_has_no_five_second_execution_deadline() {
    let (out, _) = run_at(&["slow"], false, None).await;
    assert_eq!(out.status.code(), Some(7));
    assert_eq!(out.stdout, b"out\0\xff");
}
