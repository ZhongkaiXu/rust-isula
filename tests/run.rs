use std::fs::OpenOptions;
use std::io::{Read, Seek, SeekFrom, Write};
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, Mutex};

use risula::grpc::proto;
use serde_json::{json, Value};
use tokio::net::{TcpListener, UnixListener};
use tokio_stream::wrappers::{ReceiverStream, TcpListenerStream, UnixListenerStream};
use tonic::{Request, Response, Status};

#[derive(Clone, Default, Debug)]
struct Calls {
    order: Vec<&'static str>,
    creates: Vec<proto::CreateRequest>,
    starts: Vec<proto::StartRequest>,
    waits: Vec<proto::WaitRequest>,
    metadata: Vec<(String, String)>,
    input: Vec<u8>,
    eof: bool,
    resizes: Vec<proto::ResizeRequest>,
}

#[derive(Clone)]
struct FakeRun {
    calls: Arc<Mutex<Calls>>,
    scenario: &'static str,
}

#[tonic::async_trait]
impl proto::container_service_server::ContainerService for FakeRun {
    async fn list(
        &self,
        _: Request<proto::ListRequest>,
    ) -> Result<Response<proto::ListResponse>, Status> {
        Err(Status::unimplemented("list"))
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
        _: Request<proto::InspectContainerRequest>,
    ) -> Result<Response<proto::InspectContainerResponse>, Status> {
        panic!("run already has the created container ID");
    }
    async fn resize(
        &self,
        request: Request<proto::ResizeRequest>,
    ) -> Result<Response<proto::ResizeResponse>, Status> {
        self.calls
            .lock()
            .unwrap()
            .resizes
            .push(request.into_inner());
        Ok(Response::new(proto::ResizeResponse::default()))
    }

    async fn create(
        &self,
        request: Request<proto::CreateRequest>,
    ) -> Result<Response<proto::CreateResponse>, Status> {
        let mut calls = self.calls.lock().unwrap();
        calls.order.push("create");
        calls.creates.push(request.into_inner());
        Ok(Response::new(proto::CreateResponse {
            id: if self.scenario == "no-id" {
                ""
            } else {
                "new-container"
            }
            .into(),
            cc: if self.scenario == "create-fail" { 2 } else { 0 },
            errmsg: "create rejected".into(),
            ..Default::default()
        }))
    }

    async fn start(
        &self,
        request: Request<proto::StartRequest>,
    ) -> Result<Response<proto::StartResponse>, Status> {
        let request = request.into_inner();
        {
            let mut calls = self.calls.lock().unwrap();
            calls.order.push("start");
            calls.starts.push(request.clone());
        }
        let message = match self.scenario {
            "start-fail" => "start rejected",
            "not-found" => "executable file not found",
            "permission" => "permission denied",
            _ => "",
        };
        if !message.is_empty() {
            return Ok(Response::new(proto::StartResponse {
                cc: 2,
                errmsg: message.into(),
                ..Default::default()
            }));
        }
        if self.scenario == "slow" {
            tokio::time::sleep(std::time::Duration::from_secs(6)).await;
        }
        if request.attach_stdout {
            // Open before replying to Start, just as the daemon does.
            let mut out = OpenOptions::new()
                .write(true)
                .open(&request.stdout)
                .unwrap();
            let mut err = OpenOptions::new()
                .write(true)
                .open(&request.stderr)
                .unwrap();
            let mut input = if request.attach_stdin {
                Some(OpenOptions::new().read(true).open(&request.stdin).unwrap())
            } else {
                None
            };
            let fake = self.clone();
            tokio::task::spawn_blocking(move || {
                if fake.scenario == "tty" {
                    std::thread::sleep(std::time::Duration::from_secs(3));
                }
                if matches!(fake.scenario, "input" | "file-input") {
                    let mut bytes = vec![0; 128 * 1024];
                    input.as_mut().unwrap().read_exact(&mut bytes).unwrap();
                    fake.calls.lock().unwrap().input = bytes.clone();
                    out.write_all(&bytes).unwrap();
                } else {
                    out.write_all(&b"out\0\xff".repeat(20000)).unwrap();
                    err.write_all(&b"err\0\xfe".repeat(20000)).unwrap();
                }
            });
        }
        Ok(Response::new(proto::StartResponse::default()))
    }

    async fn wait(
        &self,
        request: Request<proto::WaitRequest>,
    ) -> Result<Response<proto::WaitResponse>, Status> {
        let mut calls = self.calls.lock().unwrap();
        calls.order.push("wait");
        calls.waits.push(request.into_inner());
        Ok(Response::new(proto::WaitResponse {
            cc: if self.scenario == "wait-fail" { 2 } else { 0 },
            errmsg: "wait rejected".into(),
            exit_code: 7,
        }))
    }

    type RemoteStartStream = ReceiverStream<Result<proto::RemoteStartResponse, Status>>;
    async fn remote_start(
        &self,
        request: Request<tonic::Streaming<proto::RemoteStartRequest>>,
    ) -> Result<Response<Self::RemoteStartStream>, Status> {
        let attach = request.metadata().get("attach-stdout").unwrap() == "true";
        {
            let mut calls = self.calls.lock().unwrap();
            calls.order.push("remote_start");
            for key in [
                "container-id",
                "attach-stdin",
                "attach-stdout",
                "attach-stderr",
            ] {
                calls.metadata.push((
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
        }
        if self.scenario == "remote-header-fail" {
            let (sender, receiver) = tokio::sync::mpsc::channel(1);
            drop(sender);
            let mut response = Response::new(ReceiverStream::new(receiver));
            response.metadata_mut().insert("cc", "2".parse().unwrap());
            response
                .metadata_mut()
                .insert("errmsg", "remote start rejected".parse().unwrap());
            return Ok(response);
        }
        let fake = self.clone();
        let mut input = request.into_inner();
        let (sender, receiver) = tokio::sync::mpsc::channel(1);
        tokio::spawn(async move {
            if fake.scenario == "remote-fail" {
                let mut trailers = tonic::metadata::MetadataMap::new();
                trailers.insert("cc", "2".parse().unwrap());
                trailers.insert("errmsg", "remote rejected".parse().unwrap());
                sender
                    .send(Err(Status::with_metadata(tonic::Code::Ok, "", trailers)))
                    .await
                    .unwrap();
                return;
            }
            if fake.scenario == "input" {
                while let Some(frame) = input.message().await.unwrap() {
                    {
                        let mut calls = fake.calls.lock().unwrap();
                        calls.input.extend_from_slice(&frame.stdin);
                        calls.eof = frame.finish;
                    }
                    if frame.finish {
                        break;
                    }
                    sender
                        .send(Ok(proto::RemoteStartResponse {
                            stdout: frame.stdin,
                            ..Default::default()
                        }))
                        .await
                        .unwrap();
                }
            } else if attach {
                sender
                    .send(Ok(proto::RemoteStartResponse {
                        stdout: b"out\0\xff".repeat(20000),
                        stderr: b"err\0\xfe".repeat(20000),
                        finish: false,
                    }))
                    .await
                    .unwrap();
            }
            sender
                .send(Ok(proto::RemoteStartResponse {
                    finish: true,
                    ..Default::default()
                }))
                .await
                .unwrap();
            // Trailers are sent only after the client closes its request stream.
            while input.message().await.unwrap().is_some() {}
        });
        Ok(Response::new(ReceiverStream::new(receiver)))
    }
}

async fn run(
    args: &[&str],
    tcp: bool,
    scenario: &'static str,
    input: Option<Vec<u8>>,
) -> (Output, Calls) {
    let directory = tempfile::tempdir().unwrap();
    let calls = Arc::new(Mutex::new(Calls::default()));
    let service = proto::container_service_server::ContainerServiceServer::new(FakeRun {
        calls: calls.clone(),
        scenario,
    });
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
        let socket = directory.path().join("daemon.sock");
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
    let args: Vec<_> = args.iter().map(|s| s.to_string()).collect();
    let output = tokio::task::spawn_blocking(move || {
        if scenario == "file-input" {
            let mut file = tempfile::tempfile().unwrap();
            file.write_all(b"skip").unwrap();
            file.write_all(&input.unwrap()).unwrap();
            file.seek(SeekFrom::Start(4)).unwrap();
            return Command::new("timeout")
                .args(["20", env!("CARGO_BIN_EXE_risula"), "run", "-H", &host])
                .args(args)
                .stdin(file)
                .output()
                .unwrap();
        }
        let mut child = Command::new("timeout")
            .args(["20", env!("CARGO_BIN_EXE_risula"), "run", "-H", &host])
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        let mut stdin = child.stdin.take().unwrap();
        if let Some(input) = input {
            std::thread::spawn(move || stdin.write_all(&input).unwrap());
            child.wait_with_output().unwrap()
        } else {
            // Keep an idle input pipe open while the process exits.
            let output = child.wait_with_output().unwrap();
            drop(stdin);
            output
        }
    })
    .await
    .unwrap();
    server.abort();
    let calls = calls.lock().unwrap().clone();
    (output, calls)
}

fn check_cleanup(calls: &Calls) {
    for start in &calls.starts {
        for path in [&start.stdin, &start.stdout, &start.stderr] {
            if !path.is_empty() {
                assert!(!std::path::Path::new(path).exists(), "{path}");
            }
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn foreground_streams_binary_output_and_returns_exit_code() {
    for tcp in [false, true] {
        let (out, calls) = run(&["example:test", "sh", "-c", "exit 7"], tcp, "ok", None).await;
        assert_eq!(out.status.code(), Some(7), "{tcp}: {out:?}");
        assert_eq!(out.stdout, b"out\0\xff".repeat(20000));
        assert_eq!(out.stderr, b"err\0\xfe".repeat(20000));
        assert_eq!(calls.waits[0].id, "new-container");
        assert_eq!(calls.waits[0].condition, 0);
        let config: Value = serde_json::from_str(&calls.creates[0].customconfig).unwrap();
        assert_eq!(config["AttachStdout"], true);
        assert_eq!(config["AttachStderr"], true);
        assert_eq!(config["Cmd"], json!(["sh", "-c", "exit 7"]));
        check_cleanup(&calls);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn detached_disables_attachment_and_does_not_wait() {
    for tcp in [false, true] {
        let (out, calls) = run(&["-dit", "--rm", "example:test"], tcp, "ok", None).await;
        assert_eq!(out.status.code(), Some(0), "{tcp}: {out:?}");
        assert_eq!(out.stdout, b"new-container\n");
        assert!(out.stderr.is_empty());
        assert!(calls.waits.is_empty());
        let c: Value = serde_json::from_str(&calls.creates[0].customconfig).unwrap();
        assert_eq!(c["OpenStdin"], true);
        assert_eq!(c["Tty"], true);
        for key in ["AttachStdin", "AttachStdout", "AttachStderr"] {
            assert!(c.get(key).is_none());
        }
        let h: Value = serde_json::from_str(&calls.creates[0].hostconfig).unwrap();
        assert_eq!(h["AutoRemove"], true);
        assert_eq!(h["AutoRemoveBak"], true);
        if tcp {
            assert_eq!(
                calls.metadata[1..]
                    .iter()
                    .map(|v| v.1.as_str())
                    .collect::<Vec<_>>(),
                ["false"; 3]
            );
        } else {
            let r = &calls.starts[0];
            assert!(!r.attach_stdin && !r.attach_stdout && !r.attach_stderr);
            assert!(r.stdin.is_empty() && r.stdout.is_empty() && r.stderr.is_empty());
        }
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn interactive_input_handles_backpressure_and_tcp_eof() {
    let bytes: Vec<_> = (0..128 * 1024).map(|n| (n % 256) as u8).collect();
    for tcp in [false, true] {
        let (out, calls) = run(&["-i", "example:test"], tcp, "input", Some(bytes.clone())).await;
        assert_eq!(
            out.status.code(),
            Some(7),
            "{tcp}: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        assert_eq!(calls.input, bytes);
        assert_eq!(out.stdout, bytes);
        assert_eq!(calls.eof, tcp);
        check_cleanup(&calls);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn idle_stdin_does_not_block_exit_and_rm_waits_for_removal() {
    for tcp in [false, true] {
        let (out, calls) = run(&["-i", "--rm", "example:test"], tcp, "ok", None).await;
        assert_eq!(out.status.code(), Some(7), "{tcp}: {out:?}");
        assert_eq!(calls.waits[0].condition, 1);
        check_cleanup(&calls);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn failures_keep_native_exit_codes_and_stop_the_sequence() {
    for (scenario, code, order) in [
        ("create-fail", 1, vec!["create"]),
        ("no-id", 1, vec!["create"]),
        ("start-fail", 125, vec!["create", "start"]),
        ("not-found", 127, vec!["create", "start"]),
        ("permission", 126, vec!["create", "start"]),
        ("wait-fail", 125, vec!["create", "start", "wait"]),
    ] {
        let (out, calls) = run(&["example:test"], false, scenario, None).await;
        assert_eq!(out.status.code(), Some(code), "{scenario}: {out:?}");
        assert_eq!(calls.order, order);
        check_cleanup(&calls);
    }
    let (out, calls) = run(&["-d", "example:test"], false, "not-found", None).await;
    assert_eq!(out.status.code(), Some(127));
    assert_eq!(out.stdout, b"new-container\n");
    assert!(calls.waits.is_empty());
    let (out, calls) = run(&["example:test"], true, "remote-fail", None).await;
    assert_eq!(out.status.code(), Some(125), "{out:?}");
    assert_eq!(calls.order, ["create", "remote_start"]);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn boolean_overrides_and_validation_happen_before_creation() {
    for args in [
        vec![],
        vec!["--detach=invalid", "example:test"],
        vec!["--rm", "--restart=always", "example:test"],
        vec!["-a", "example:test"],
    ] {
        let (out, calls) = run(&args, false, "ok", None).await;
        assert_eq!(out.status.code(), Some(125), "{args:?}: {out:?}");
        assert!(calls.order.is_empty());
    }
    let (out, calls) = run(
        &[
            "-dit",
            "--detach=false",
            "--interactive=false",
            "--tty=false",
            "example:test",
            "--detach",
        ],
        false,
        "ok",
        None,
    )
    .await;
    assert_eq!(out.status.code(), Some(7));
    let c: Value = serde_json::from_str(&calls.creates[0].customconfig).unwrap();
    assert_eq!(c["AttachStdout"], true);
    assert!(c.get("OpenStdin").is_none());
    assert!(c.get("Tty").is_none());
    assert_eq!(c["Cmd"], json!(["--detach"]));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn start_has_no_five_second_execution_deadline() {
    let (out, calls) = run(&["example:test"], false, "slow", None).await;
    assert_eq!(out.status.code(), Some(7), "{out:?}");
    check_cleanup(&calls);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn redirected_file_input_preserves_its_current_position() {
    let bytes = b"file input\n"
        .iter()
        .copied()
        .cycle()
        .take(128 * 1024)
        .collect::<Vec<_>>();
    let (out, calls) = run(
        &["-i", "example:test"],
        false,
        "file-input",
        Some(bytes.clone()),
    )
    .await;
    assert_eq!(
        out.status.code(),
        Some(7),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(calls.input, bytes);
    assert_eq!(out.stdout, bytes);
    check_cleanup(&calls);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn remote_errors_in_initial_headers_are_not_lost() {
    for args in [vec!["example:test"], vec!["-d", "example:test"]] {
        let (out, calls) = run(&args, true, "remote-header-fail", None).await;
        assert_eq!(out.status.code(), Some(125), "{out:?}");
        assert!(String::from_utf8_lossy(&out.stderr).contains("remote start rejected"));
        assert_eq!(calls.order, ["create", "remote_start"]);
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn tty_resizes_and_restores_settings_on_success_and_start_failure() {
    use std::os::fd::FromRawFd;
    for scenario in ["tty", "not-found"] {
        let directory = tempfile::tempdir().unwrap();
        let socket = directory.path().join("daemon.sock");
        let listener = UnixListener::bind(&socket).unwrap();
        let calls = Arc::new(Mutex::new(Calls::default()));
        let fake = FakeRun {
            calls: calls.clone(),
            scenario,
        };
        let server = tokio::spawn(async move {
            tonic::transport::Server::builder()
                .add_service(proto::container_service_server::ContainerServiceServer::new(fake))
                .serve_with_incoming(UnixListenerStream::new(listener))
                .await
                .unwrap();
        });
        let output = tokio::task::spawn_blocking(move || {
            let (mut master, mut slave) = (-1, -1);
            // SAFETY: openpty writes two descriptors; all optional arguments are null.
            assert_eq!(
                unsafe {
                    libc::openpty(
                        &mut master,
                        &mut slave,
                        std::ptr::null_mut(),
                        std::ptr::null(),
                        std::ptr::null(),
                    )
                },
                0
            );
            let _master = unsafe { std::fs::File::from_raw_fd(master) };
            let slave_file = unsafe { std::fs::File::from_raw_fd(slave) };
            let mut original: libc::termios = unsafe { std::mem::zeroed() };
            assert_eq!(unsafe { libc::tcgetattr(slave, &mut original) }, 0);
            let size = libc::winsize {
                ws_row: 24,
                ws_col: 80,
                ws_xpixel: 0,
                ws_ypixel: 0,
            };
            assert_eq!(unsafe { libc::ioctl(slave, libc::TIOCSWINSZ, &size) }, 0);
            let child = Command::new("timeout")
                .args([
                    "15",
                    env!("CARGO_BIN_EXE_risula"),
                    "run",
                    "-H",
                    &format!("unix://{}", socket.display()),
                    "-it",
                    "example:test",
                ])
                .stdin(slave_file.try_clone().unwrap())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .unwrap();
            if scenario == "tty" {
                std::thread::sleep(std::time::Duration::from_millis(1400));
                let mut raw: libc::termios = unsafe { std::mem::zeroed() };
                assert_eq!(unsafe { libc::tcgetattr(slave, &mut raw) }, 0);
                assert_eq!(raw.c_lflag & (libc::ICANON | libc::ECHO), 0);
                let size = libc::winsize {
                    ws_row: 40,
                    ws_col: 100,
                    ..size
                };
                assert_eq!(unsafe { libc::ioctl(slave, libc::TIOCSWINSZ, &size) }, 0);
            }
            let output = child.wait_with_output().unwrap();
            let mut restored: libc::termios = unsafe { std::mem::zeroed() };
            assert_eq!(unsafe { libc::tcgetattr(slave, &mut restored) }, 0);
            assert_eq!(restored.c_iflag, original.c_iflag);
            assert_eq!(restored.c_oflag, original.c_oflag);
            assert_eq!(restored.c_cflag, original.c_cflag);
            assert_eq!(restored.c_lflag, original.c_lflag);
            assert_eq!(restored.c_cc, original.c_cc);
            output
        })
        .await
        .unwrap();
        server.abort();
        let calls = calls.lock().unwrap();
        assert_eq!(
            output.status.code(),
            Some(if scenario == "tty" { 7 } else { 127 }),
            "{output:?}"
        );
        if scenario == "tty" {
            let sizes: Vec<_> = calls
                .resizes
                .iter()
                .map(|r| {
                    assert_eq!(r.id, "new-container");
                    assert!(r.suffix.is_empty());
                    (r.height, r.width)
                })
                .collect();
            assert_eq!(sizes, [(24, 80), (40, 100)]);
        }
        check_cleanup(&calls);
    }
}
