use std::fs::OpenOptions;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

use risula::grpc::proto::{self, StartRequest};
use tokio::net::{TcpListener, UnixListener};
use tokio_stream::wrappers::{TcpListenerStream, UnixListenerStream};
use tonic::{Request, Response, Status};

#[derive(Default, Debug)]
struct Calls {
    starts: Vec<StartRequest>,
    inspections: Vec<proto::InspectContainerRequest>,
    waits: Vec<proto::WaitRequest>,
    remote: Vec<Vec<(String, String)>>,
}

struct FakeStart(Arc<Mutex<Calls>>);

#[tonic::async_trait]
impl proto::container_service_server::ContainerService for FakeStart {
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
        _: Request<proto::RenameRequest>,
    ) -> Result<Response<proto::RenameResponse>, Status> {
        Err(Status::unimplemented("rename"))
    }

    async fn inspect(
        &self,
        request: Request<proto::InspectContainerRequest>,
    ) -> Result<Response<proto::InspectContainerResponse>, Status> {
        let request = request.into_inner();
        self.0.lock().unwrap().inspections.push(request.clone());
        let mut response = proto::InspectContainerResponse::default();
        response.container_json = match request.id.as_str() {
            "missing" => {
                response.cc = 2;
                response.errmsg = "No such container".into();
                String::new()
            }
            "bad-json" => "{".into(),
            "no-id" => "{}".into(),
            _ => serde_json::json!({"Id": format!("id-{}", request.id)}).to_string(),
        };
        Ok(Response::new(response))
    }

    async fn start(
        &self,
        request: Request<StartRequest>,
    ) -> Result<Response<proto::StartResponse>, Status> {
        let request = request.into_inner();
        self.0.lock().unwrap().starts.push(request.clone());
        let name = request.id.strip_prefix("id-").unwrap_or(&request.id);
        let message = match name {
            "missing" | "failure" => "No such container",
            "not-found" => "Executable file not found",
            "permission" => "Permission denied",
            "not-directory" => "Not a directory",
            _ => "",
        };
        if !message.is_empty() {
            return Ok(Response::new(proto::StartResponse {
                cc: 2,
                errmsg: message.into(),
                ..Default::default()
            }));
        }
        if name == "unavailable" {
            return Err(Status::unavailable("daemon unavailable"));
        }
        if name == "internal" {
            return Err(Status::internal("daemon internal error"));
        }
        if name == "slow" {
            tokio::time::sleep(std::time::Duration::from_secs(6)).await;
        }
        if request.attach_stdout {
            for (path, data) in [
                (&request.stdout, b"out\0\xff"),
                (&request.stderr, b"err\0\xfe"),
            ] {
                let mut fifo = OpenOptions::new().write(true).open(path).unwrap();
                let repeats = if name == "large" { 32768 } else { 1 };
                fifo.write_all(&data.repeat(repeats)).unwrap();
            }
        }
        Ok(Response::new(proto::StartResponse {
            id: "ignored-response-id".into(),
            errmsg: "ignored success message".into(),
            ..Default::default()
        }))
    }

    async fn wait(
        &self,
        request: Request<proto::WaitRequest>,
    ) -> Result<Response<proto::WaitResponse>, Status> {
        let request = request.into_inner();
        self.0.lock().unwrap().waits.push(request.clone());
        if request.id.contains("wait-error") {
            return Err(Status::unavailable("wait failed"));
        }
        if request.id.contains("wait-business") {
            return Ok(Response::new(proto::WaitResponse {
                cc: 2,
                errmsg: "wait rejected".into(),
                ..Default::default()
            }));
        }
        if request.id.contains("wait-slow") {
            tokio::time::sleep(std::time::Duration::from_secs(6)).await;
        }
        Ok(Response::new(proto::WaitResponse {
            exit_code: if request.id.contains("exit-seven") {
                7
            } else {
                0
            },
            ..Default::default()
        }))
    }

    type RemoteStartStream =
        tokio_stream::Iter<std::vec::IntoIter<Result<proto::RemoteStartResponse, Status>>>;

    async fn remote_start(
        &self,
        request: Request<tonic::Streaming<proto::RemoteStartRequest>>,
    ) -> Result<Response<Self::RemoteStartStream>, Status> {
        let mut metadata = Vec::new();
        for key in [
            "container-id",
            "attach-stdin",
            "attach-stdout",
            "attach-stderr",
        ] {
            metadata.push((
                key.into(),
                request
                    .metadata()
                    .get(key)
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .into(),
            ));
        }
        let name: String = request
            .metadata()
            .get("container-id")
            .unwrap()
            .to_str()
            .unwrap()
            .into();
        self.0.lock().unwrap().remote.push(metadata);
        if name == "failure" {
            return Err(Status::unavailable("remote start failed"));
        }
        if name == "remote-business" || name == "remote-bad-status" {
            let mut trailers = tonic::metadata::MetadataMap::new();
            let cc = if name == "remote-business" {
                "2"
            } else {
                "bad"
            };
            trailers.insert("cc", cc.parse().unwrap());
            trailers.insert("errmsg", "remote start rejected".parse().unwrap());
            // An OK gRPC status with business status in trailing metadata matches iSulad.
            return Ok(Response::new(tokio_stream::iter(vec![Err(
                Status::with_metadata(tonic::Code::Ok, "", trailers),
            )])));
        }
        let mut frames = vec![Ok(proto::RemoteStartResponse {
            stdout: b"out\0\xff".to_vec(),
            stderr: b"err\0\xfe".to_vec(),
            finish: false,
        })];
        if name == "stream-error" {
            frames.push(Err(Status::internal("stream failed")));
        } else {
            frames.push(Ok(proto::RemoteStartResponse {
                finish: true,
                ..Default::default()
            }));
        }
        Ok(Response::new(tokio_stream::iter(frames)))
    }
}

async fn run_start(args: &[&str], tcp: bool) -> (Output, Calls) {
    let directory = tempfile::tempdir().unwrap();
    let calls = Arc::new(Mutex::new(Calls::default()));
    let service =
        proto::container_service_server::ContainerServiceServer::new(FakeStart(calls.clone()));
    let (host, server) = if tcp {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let host = format!("tcp://{}", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_service(service)
                .serve_with_incoming(TcpListenerStream::new(listener))
                .await
                .unwrap();
        });
        (host, server)
    } else {
        let socket = directory.path().join("isulad.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let host = format!("unix://{}", socket.display());
        let server = tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_service(service)
                .serve_with_incoming(UnixListenerStream::new(listener))
                .await
                .unwrap();
        });
        (host, server)
    };
    let args: Vec<String> = args.iter().map(|arg| arg.to_string()).collect();
    let output = tokio::task::spawn_blocking(move || {
        Command::new("timeout")
            .args(["25", env!("CARGO_BIN_EXE_risula")])
            .env("ISULAD_HOST", "unix:///unused.sock")
            .args(["start", "-H", &host])
            .args(args)
            .output()
            .unwrap()
    })
    .await
    .unwrap();
    server.abort();
    let calls = std::mem::take(&mut *calls.lock().unwrap());
    for request in &calls.starts {
        if request.attach_stdout {
            assert!(
                !Path::new(&request.stdout).parent().unwrap().exists(),
                "FIFO directory leaked"
            );
        }
    }
    (output, calls)
}

#[tokio::test]
async fn start_sends_names_and_ids_without_output_or_attach() {
    let full = "a".repeat(64);
    for tcp in [false, true] {
        let (out, calls) = run_start(&["name", "123456789abc", &full, "name"], tcp).await;
        assert!(out.status.success(), "{out:?}");
        assert!(out.stdout.is_empty() && out.stderr.is_empty());
        assert_eq!(calls.starts.len(), 4);
        for (request, name) in calls
            .starts
            .iter()
            .zip(["name", "123456789abc", &full, "name"])
        {
            assert_eq!(
                *request,
                StartRequest {
                    id: name.into(),
                    ..Default::default()
                }
            );
        }
        assert!(calls.inspections.is_empty() && calls.waits.is_empty() && calls.remote.is_empty());
    }
}

#[tokio::test]
async fn start_continues_after_business_transport_and_empty_name_errors() {
    let (out, calls) = run_start(&["missing", "good", "", "unavailable", "last"], false).await;
    assert_eq!(out.status.code(), Some(1));
    assert!(out.stdout.is_empty());
    assert_eq!(
        calls
            .starts
            .iter()
            .map(|req| req.id.as_str())
            .collect::<Vec<_>>(),
        ["missing", "good", "unavailable", "last"]
    );
    let stderr = String::from_utf8(out.stderr).unwrap();
    assert!(
        stderr.contains("No such container")
            && stderr.contains("empty")
            && stderr.contains("daemon unavailable")
    );
}

#[tokio::test]
async fn start_validates_arguments_flags_and_attach_batch_before_rpc() {
    let mut cases = vec![
        vec![],
        vec!["--unknown", "name"],
        vec!["-i", "name"],
        vec!["-H"],
        vec!["--attach=bad", "name"],
        vec!["-a", "one", "two"],
    ];
    cases.push(vec!["name"; 1000]);
    for args in cases {
        let (out, calls) = run_start(&args, false).await;
        assert_eq!(out.status.code(), Some(125), "{args:?}: {out:?}");
        assert!(calls.starts.is_empty() && calls.inspections.is_empty());
    }
    for args in [
        vec!["name"; 999],
        vec!["--attach=false", "one", "two"],
        vec!["-a", "--attach=false", "one", "two"],
        vec!["--", "-name"],
    ] {
        let (out, calls) = run_start(&args, false).await;
        assert!(out.status.success(), "{out:?}");
        assert!(!calls.starts.is_empty() && calls.inspections.is_empty());
    }
    let (out, calls) = run_start(&["--help"], false).await;
    assert!(out.status.success());
    assert!(String::from_utf8(out.stdout).unwrap().contains("--attach"));
    assert!(calls.starts.is_empty());
}

#[tokio::test]
async fn start_attach_streams_binary_output_and_returns_container_exit_code() {
    for (option, name, code, repeats) in [
        ("-a", "exit-seven", 7, 1),
        ("--attach=true", "large", 0, 32768),
    ] {
        let (out, calls) = run_start(&[option, name], false).await;
        assert_eq!(out.status.code(), Some(code), "{out:?}");
        assert_eq!(out.stdout, b"out\0\xff".repeat(repeats));
        assert_eq!(out.stderr, b"err\0\xfe".repeat(repeats));
        assert_eq!(
            calls.inspections,
            [proto::InspectContainerRequest {
                id: name.into(),
                timeout: 120,
                bformat: false
            }]
        );
        let request = &calls.starts[0];
        assert_eq!(request.id, format!("id-{name}"));
        assert!(request.attach_stdout && request.attach_stderr && !request.attach_stdin);
        assert!(request.stdin.is_empty());
        assert_eq!(
            calls.waits,
            [proto::WaitRequest {
                id: request.id.clone(),
                condition: 0
            }]
        );
    }
}

#[tokio::test]
async fn start_attach_inspect_errors_do_not_start() {
    for name in ["missing", "bad-json", "no-id", ""] {
        let (out, calls) = run_start(&["-a", name], false).await;
        assert_eq!(out.status.code(), Some(255), "{out:?}");
        assert!(calls.starts.is_empty() && calls.waits.is_empty());
        if name.is_empty() {
            assert!(calls.inspections.is_empty());
        }
    }
}

#[tokio::test]
async fn start_attach_maps_start_and_wait_failures_and_cleans_fifos() {
    for (name, code) in [
        ("failure", 125),
        ("not-found", 127),
        ("permission", 126),
        ("not-directory", 127),
        ("unavailable", 125),
        ("internal", 1),
        ("wait-error", 125),
        ("wait-business", 125),
    ] {
        let (out, calls) = run_start(&["-a", name], false).await;
        assert_eq!(out.status.code(), Some(code), "{name}: {out:?}");
        assert_eq!(calls.starts.len(), 1);
        assert_eq!(calls.waits.len(), usize::from(name.starts_with("wait-")));
    }
}

#[tokio::test]
async fn start_and_wait_allow_slow_daemon_responses() {
    for name in ["slow", "wait-slow"] {
        let (out, _) = run_start(&["-a", name], false).await;
        assert!(out.status.success(), "{out:?}");
        assert_eq!(out.stdout, b"out\0\xff");
    }
}

#[tokio::test]
async fn start_tcp_attach_uses_stream_metadata_then_wait() {
    let (out, calls) = run_start(&["-a", "exit-seven"], true).await;
    assert_eq!(out.status.code(), Some(7), "{out:?}");
    assert_eq!(out.stdout, b"out\0\xff");
    assert_eq!(out.stderr, b"err\0\xfe");
    assert!(calls.starts.is_empty() && calls.inspections.is_empty());
    assert_eq!(
        calls.remote,
        [vec![
            ("container-id".into(), "exit-seven".into()),
            ("attach-stdin".into(), "false".into()),
            ("attach-stdout".into(), "true".into()),
            ("attach-stderr".into(), "true".into())
        ]]
    );
    assert_eq!(
        calls.waits,
        [proto::WaitRequest {
            id: "exit-seven".into(),
            condition: 0
        }]
    );
}

#[tokio::test]
async fn start_tcp_attach_reports_rpc_and_stream_errors_without_waiting() {
    for (name, code) in [
        ("failure", 125),
        ("stream-error", 1),
        ("remote-business", 125),
        ("remote-bad-status", 1),
    ] {
        let (out, calls) = run_start(&["-a", name], true).await;
        assert_eq!(out.status.code(), Some(code), "{out:?}");
        assert!(calls.waits.is_empty());
        if name == "remote-business" {
            assert!(String::from_utf8(out.stderr)
                .unwrap()
                .contains("remote start rejected"));
        }
    }
}

#[test]
fn start_reports_connection_failure() {
    let directory = tempfile::tempdir().unwrap();
    for (args, code) in [
        (vec!["start", "name"], 1),
        (vec!["start", "-a", "name"], 255),
    ] {
        let output = Command::new(env!("CARGO_BIN_EXE_risula"))
            .env(
                "ISULAD_HOST",
                format!("unix://{}/missing.sock", directory.path().display()),
            )
            .args(args)
            .output()
            .unwrap();
        assert_eq!(output.status.code(), Some(code));
        assert!(output.stdout.is_empty());
    }
}
