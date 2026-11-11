use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

use risula::grpc::proto::{self, ExecRequest};
use tokio::net::{TcpListener, UnixListener};
use tokio_stream::wrappers::{TcpListenerStream, UnixListenerStream};
use tonic::{Request, Response, Status};

struct FakeExec {
    calls: Arc<Mutex<Vec<ExecRequest>>>,
}

#[tonic::async_trait]
impl proto::container_service_server::ContainerService for FakeExec {
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
        request: Request<ExecRequest>,
    ) -> Result<Response<proto::ExecResponse>, Status> {
        use std::io::Write;
        let request = request.into_inner();
        self.calls.lock().unwrap().push(request.clone());
        if request.argv[0] == "fail" {
            return Ok(Response::new(proto::ExecResponse {
                cc: 1,
                exit_code: 126,
                errmsg: "permission denied".into(),
                ..Default::default()
            }));
        }
        if request.attach_stdout {
            std::fs::OpenOptions::new()
                .write(true)
                .open(&request.stdout)
                .unwrap()
                .write_all(b"out\x00\xff")
                .unwrap();
            std::fs::OpenOptions::new()
                .write(true)
                .open(&request.stderr)
                .unwrap()
                .write_all(b"err")
                .unwrap();
        }
        Ok(Response::new(proto::ExecResponse {
            exit_code: 7,
            ..Default::default()
        }))
    }
    type RemoteExecStream =
        tokio_stream::Iter<std::vec::IntoIter<Result<proto::RemoteExecResponse, Status>>>;
    async fn remote_exec(
        &self,
        request: Request<tonic::Streaming<proto::RemoteExecRequest>>,
    ) -> Result<Response<Self::RemoteExecStream>, Status> {
        let json: serde_json::Value = serde_json::from_str(
            request
                .metadata()
                .get("isulad-remote-exec")
                .unwrap()
                .to_str()
                .unwrap(),
        )
        .unwrap();
        self.calls.lock().unwrap().push(ExecRequest {
            container_id: json["container_id"].as_str().unwrap().into(),
            argv: json["argv"]
                .as_array()
                .unwrap()
                .iter()
                .map(|s| s.as_str().unwrap().into())
                .collect(),
            workdir: json["workdir"].as_str().unwrap_or_default().into(),
            user: json["user"].as_str().unwrap_or_default().into(),
            ..Default::default()
        });
        if json["argv"][0] == "defaults" {
            assert!(json.get("workdir").is_none());
            assert!(json.get("user").is_none());
        }
        let failure = json["argv"][0] == "fail";
        let mut response = Response::new(tokio_stream::iter(vec![Ok(proto::RemoteExecResponse {
            stdout: b"remote\x00\xff".to_vec(),
            stderr: b"stderr".to_vec(),
            finish: true,
        })]));
        response
            .metadata_mut()
            .insert("cc", if failure { "1" } else { "0" }.parse().unwrap());
        response
            .metadata_mut()
            .insert("exit_code", "23".parse().unwrap());
        response
            .metadata_mut()
            .insert("errmsg", "remote failure".parse().unwrap());
        Ok(response)
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
        let name = request.into_inner().id;
        Ok(Response::new(proto::InspectContainerResponse {
            container_json: format!(r#"{{"Id":"{name}"}}"#),
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

async fn run_exec(args: &[&str]) -> (Output, Vec<ExecRequest>) {
    run_at(args, false).await
}

async fn run_at(args: &[&str], tcp: bool) -> (Output, Vec<ExecRequest>) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("isulad.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let tcp_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = tcp_listener.local_addr().unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let server_calls = calls.clone();
    let server = tokio::spawn(async move {
        let service = proto::container_service_server::ContainerServiceServer::new(FakeExec {
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
        Command::new(env!("CARGO_BIN_EXE_risula"))
            .args(["exec", "-H", &host])
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
async fn exec_preserves_output_exit_code_and_cleans_fifos() {
    let (out, calls) = run_exec(&["demo", "sh", "-c", "echo hello"]).await;
    assert_eq!(out.status.code(), Some(7));
    assert_eq!(out.stdout, b"out\x00\xff");
    assert_eq!(out.stderr, b"err");
    assert_eq!(calls[0].argv, ["sh", "-c", "echo hello"]);
    assert_eq!(calls[0].suffix.len(), 64);
    assert!(!std::path::Path::new(&calls[0].stdout).exists());
}

#[tokio::test]
async fn exec_detach_clears_attach_flags_and_preserves_configuration() {
    let (out, calls) = run_exec(&[
        "-dit",
        "-e",
        "DEMO=hello",
        "-u",
        "12:34",
        "--workdir",
        "/tmp",
        "demo",
        "true",
    ])
    .await;
    assert_eq!(out.status.code(), Some(7));
    assert!(out.stdout.is_empty());
    let request = &calls[0];
    assert!(request.tty);
    assert!(
        !request.open_stdin
            && !request.attach_stdin
            && !request.attach_stdout
            && !request.attach_stderr
    );
    assert_eq!(request.env, ["DEMO=hello"]);
    assert_eq!(request.user, "12:34");
    assert_eq!(request.workdir, "/tmp");
    assert!(request.stdout.is_empty());
}

#[tokio::test]
async fn exec_reports_daemon_failure_without_waiting_for_fifo_eof() {
    let (out, calls) = run_exec(&["demo", "fail"]).await;
    assert_eq!(out.status.code(), Some(126));
    assert!(String::from_utf8_lossy(&out.stderr).contains("permission denied"));
    assert!(!std::path::Path::new(&calls[0].stdout).exists());
}

#[tokio::test]
async fn exec_rejects_invalid_options_before_rpc() {
    for args in [
        vec![],
        vec!["demo"],
        vec!["--workdir", "relative", "demo", "true"],
        vec!["-e", "=bad", "demo", "true"],
        vec!["--tty=bad", "demo", "true"],
    ] {
        let (out, calls) = run_exec(&args).await;
        assert!(!out.status.success());
        assert!(calls.is_empty());
    }
}

#[tokio::test]
async fn remote_exec_preserves_unicode_and_reads_status_from_headers() {
    for program in ["echo", "fail"] {
        let (out, calls) = run_at(
            &["--workdir", "/tmp", "-u", "123", "demo", program, "你好🌍"],
            true,
        )
        .await;
        assert_eq!(out.status.code(), Some(23));
        assert_eq!(out.stdout, b"remote\x00\xff");
        assert!(out.stderr.starts_with(b"stderr"));
        if program == "fail" {
            assert!(String::from_utf8_lossy(&out.stderr).contains("remote failure"));
        }
        assert_eq!(calls[0].argv, [program, "你好🌍"]);
        assert_eq!(calls[0].workdir, "/tmp");
        assert_eq!(calls[0].user, "123");
    }
}

#[tokio::test]
async fn remote_exec_omits_unspecified_user_and_workdir() {
    let (out, calls) = run_at(&["demo", "defaults"], true).await;
    assert_eq!(out.status.code(), Some(23));
    assert_eq!(calls.len(), 1);
}
