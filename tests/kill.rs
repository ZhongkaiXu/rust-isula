use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

use risula::grpc::proto::{self, KillRequest, KillResponse};
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{Request, Response, Status};

struct FakeKill {
    calls: Arc<Mutex<Vec<KillRequest>>>,
}

#[tonic::async_trait]
impl proto::container_service_server::ContainerService for FakeKill {
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

    async fn kill(&self, request: Request<KillRequest>) -> Result<Response<KillResponse>, Status> {
        let request = request.into_inner();
        self.calls.lock().unwrap().push(request.clone());
        if request.id == "missing" {
            return Ok(Response::new(KillResponse {
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
        Ok(Response::new(KillResponse {
            id: "resolved-container-id".into(),
            errmsg: "ignored on success".into(),
            ..Default::default()
        }))
    }
}

async fn run_kill(args: &[&str]) -> (Output, Vec<KillRequest>) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("isulad.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let server_calls = calls.clone();
    let server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(
                proto::container_service_server::ContainerServiceServer::new(FakeKill {
                    calls: server_calls,
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
            .args(["kill", "-H", &host])
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
async fn kill_defaults_to_sigkill_and_prints_original_names() {
    let (output, calls) = run_kill(&["first", "second"]).await;
    assert!(output.status.success(), "{output:?}");
    assert_eq!(
        calls,
        vec![
            KillRequest {
                id: "first".into(),
                signal: 9
            },
            KillRequest {
                id: "second".into(),
                signal: 9
            },
        ]
    );
    assert_eq!(output.stdout, b"first\nsecond\n");
    assert!(output.stderr.is_empty());
}

#[tokio::test]
async fn kill_accepts_standard_signal_names_and_aliases() {
    let names = [
        "HUP", "INT", "QUIT", "ILL", "TRAP", "ABRT", "BUS", "FPE", "KILL", "USR1", "SEGV", "USR2",
        "PIPE", "ALRM", "TERM", "STKFLT", "CHLD", "CONT", "STOP", "TSTP", "TTIN", "TTOU", "URG",
        "XCPU", "XFSZ", "VTALRM", "PROF", "WINCH", "IO", "PWR", "SYS",
    ];
    for (index, name) in names.iter().enumerate() {
        for value in [name.to_string(), format!("sIg{}", name.to_lowercase())] {
            let (output, calls) = run_kill(&["-s", &value, "test"]).await;
            assert!(output.status.success(), "{value}: {output:?}");
            assert_eq!(calls[0].signal, index as u32 + 1, "{value}");
        }
    }
    for (name, expected) in [
        ("IOT", 6),
        ("EMT", 7),
        ("CLD", 17),
        ("POLL", 29),
        ("INFO", 29),
        ("LOST", 37),
        ("UNUSED", 31),
    ] {
        let (output, calls) = run_kill(&["--signal", name, "test"]).await;
        assert!(output.status.success(), "{name}: {output:?}");
        assert_eq!(calls[0].signal, expected);
    }
}

#[tokio::test]
async fn kill_accepts_numeric_signals_and_base_zero_formats() {
    for expected in (1..=31).chain(34..=64) {
        let (output, calls) = run_kill(&["-s", &expected.to_string(), "test"]).await;
        assert!(output.status.success(), "{expected}: {output:?}");
        assert_eq!(calls[0].signal, expected);
    }
    for (value, expected) in [("017", 15), ("0xf", 15), ("0X0F", 15), ("00011", 9)] {
        let (output, calls) = run_kill(&["-s", value, "test"]).await;
        assert!(output.status.success(), "{value}: {output:?}");
        assert_eq!(calls[0].signal, expected);
    }
}

#[tokio::test]
async fn kill_accepts_exact_upstream_realtime_names() {
    let mut values = vec![("RTMIN".into(), 34), ("sigrtmax".into(), 64)];
    values.extend((1..=15).map(|n| (format!("SIGRTMIN+{n}"), 34 + n)));
    values.extend((1..=14).map(|n| (format!("RTMAX-{n}"), 64 - n)));
    for (name, expected) in values {
        let (output, calls) = run_kill(&["--signal", &name, "test"]).await;
        assert!(output.status.success(), "{name}: {output:?}");
        assert_eq!(calls[0].signal, expected);
    }
}

#[tokio::test]
async fn kill_uses_final_signal_and_matches_short_option_syntax() {
    for args in [
        vec!["--signal=TERM", "test"],
        vec!["-s=TERM", "test"],
        vec!["-Ds=TERM", "test"],
        vec!["-s", "bad", "--signal=TERM", "test"],
        vec!["-ss", "bad", "TERM", "test"],
    ] {
        let (output, calls) = run_kill(&args).await;
        assert!(output.status.success(), "{args:?}: {output:?}");
        assert_eq!(calls[0].signal, 15);
    }
}

#[tokio::test]
async fn kill_rejects_invalid_or_unsupported_signals_without_rpc() {
    for value in [
        "",
        "bad",
        "0",
        "32",
        "33",
        "65",
        "2147483648",
        "-1",
        "+9",
        " 9",
        "9 ",
        "08",
        "0x",
        "0x+9",
        "SIG9",
        "RTMIN+0",
        "RTMIN+16",
        "RTMAX-0",
        "RTMAX-15",
        "RTMIN+01",
        "RTMAX-01",
        "RTMIN++1",
    ] {
        let (output, calls) = run_kill(&["-s", value, "test"]).await;
        assert_eq!(output.status.code(), Some(1), "{value:?}: {output:?}");
        assert!(calls.is_empty());
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
    }
    let (output, calls) = run_kill(&["-s", "bad"]).await;
    assert_eq!(output.status.code(), Some(1));
    assert!(calls.is_empty());
}

#[tokio::test]
async fn kill_option_errors_have_upstream_exit_codes() {
    for (args, expected) in [
        (vec!["-s"], 1),
        (vec!["--signal"], 1),
        (vec!["--signal=", "test"], 1),
        (vec!["-s=", "test"], 1),
        (vec!["-H"], 1),
        (vec!["--host=", "test"], 1),
        (vec!["-sTERM", "test"], 125),
        (vec!["--unknown", "test"], 125),
        (vec!["-f", "test"], 125),
    ] {
        let (output, calls) = run_kill(&args).await;
        assert_eq!(output.status.code(), Some(expected), "{args:?}: {output:?}");
        assert!(calls.is_empty());
    }
}

#[tokio::test]
async fn kill_validates_container_count() {
    for args in [vec![], vec!["test"; 1000]] {
        let (output, calls) = run_kill(&args).await;
        assert_eq!(output.status.code(), Some(125));
        assert!(calls.is_empty());
    }
    let (output, calls) = run_kill(&vec!["test"; 999]).await;
    assert!(output.status.success());
    assert_eq!(calls.len(), 999);
}

#[tokio::test]
async fn kill_continues_after_business_transport_and_empty_name_errors() {
    let (output, calls) = run_kill(&["missing", "", "unavailable", "good"]).await;
    assert_eq!(output.status.code(), Some(1));
    assert_eq!(
        calls.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
        ["missing", "unavailable", "good"]
    );
    assert_eq!(output.stdout, b"good\n");
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("code 42"));
    assert!(stderr.contains("No such container:missing"));
    assert!(stderr.contains("container name cannot be empty"));
    assert!(stderr.contains("container service unavailable"));
}

#[tokio::test]
async fn kill_waits_for_slow_daemon_response() {
    let (output, calls) = run_kill(&["slow"]).await;
    assert!(output.status.success(), "{output:?}");
    assert_eq!(calls.len(), 1);
}

#[tokio::test]
async fn kill_help_and_option_terminator() {
    let (output, calls) = run_kill(&["--signal=bad", "--help"]).await;
    assert!(output.status.success());
    assert!(calls.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--signal"));
    assert!(stdout.contains("SIGKILL"));
    let (output, calls) = run_kill(&["--", "-name"]).await;
    assert!(output.status.success());
    assert_eq!(calls[0].id, "-name");
}

#[test]
fn kill_reports_connection_failure_for_each_container() {
    let directory = tempfile::tempdir().unwrap();
    let host = format!("unix://{}/missing.sock", directory.path().display());
    let output = Command::new(env!("CARGO_BIN_EXE_risula"))
        .env("ISULAD_HOST", host)
        .args(["kill", "first", "second"])
        .output()
        .unwrap();
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert_eq!(stderr.matches("cannot connect").count(), 2);
    assert!(stderr.contains("first"));
    assert!(stderr.contains("second"));
}
