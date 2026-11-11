use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

use risula::grpc::proto::{self, StatsRequest, StatsResponse};
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{Request, Response, Status};

struct FakeStats {
    calls: Arc<Mutex<Vec<StatsRequest>>>,
    replies: Mutex<std::collections::VecDeque<Result<StatsResponse, Status>>>,
}

#[tonic::async_trait]
impl proto::container_service_server::ContainerService for FakeStats {
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
        request: Request<StatsRequest>,
    ) -> Result<Response<StatsResponse>, Status> {
        self.calls.lock().unwrap().push(request.into_inner());
        self.replies
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected extra sample")
            .map(Response::new)
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
}

async fn run_stats(
    args: &[&str],
    replies: Vec<Result<StatsResponse, Status>>,
) -> (Output, Vec<StatsRequest>) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("isulad.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let server_calls = calls.clone();
    let server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(
                proto::container_service_server::ContainerServiceServer::new(FakeStats {
                    calls: server_calls,
                    replies: Mutex::new(replies.into()),
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
            .args(["stats", "-H", &host])
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

fn sample(cpu: u64, system: u64) -> Result<StatsResponse, Status> {
    Ok(StatsResponse {
        containers: vec![proto::ContainerInfo {
            id: "a".repeat(64),
            name: "demo".into(),
            status: "running".into(),
            cpu_use_nanos: cpu,
            cpu_system_use: system,
            online_cpus: 4,
            workingset_bytes: 512,
            mem_used: 900,
            mem_limit: 1024,
            blkio_read: 1024,
            blkio_write: 2048,
            pids_current: 2,
            cache: 123,
            ..Default::default()
        }],
        ..Default::default()
    })
}

#[tokio::test]
async fn stats_no_stream_samples_twice_and_calculates_percentages() {
    let (out, calls) = run_stats(
        &["--no-stream", "demo", "second"],
        vec![sample(100, 1000), sample(150, 1200)],
    )
    .await;
    assert!(out.status.success());
    assert_eq!(
        calls,
        vec![
            StatsRequest {
                containers: vec!["demo".into(), "second".into()],
                all: false
            };
            2
        ]
    );
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(text.starts_with("\x1b[H\x1b[J\x1b[7m\x1b[1mCONTAINER"));
    let row = text.lines().nth(1).unwrap().trim_start_matches("\x1b[0m");
    assert_eq!(
        row,
        format!(
            "{:<16} {:<10} {:<26} {:<10} {:<26} {:<10}",
            "aaaaaaaaaaaa", "100.00", "512.00 B / 1024.00 B", "50.00", "1024.00 B / 2.00 KiB", "2"
        )
    );
}

#[tokio::test]
async fn stats_original_uses_one_sample_and_raw_memory() {
    let (out, calls) = run_stats(&["-a", "--original"], vec![sample(100, 1000)]).await;
    assert!(out.status.success());
    assert_eq!(
        calls,
        vec![StatsRequest {
            all: true,
            containers: vec![]
        }]
    );
    let text = String::from_utf8(out.stdout).unwrap();
    assert!(!text.contains('\x1b'));
    assert_eq!(text.lines().count(), 2);
    assert_eq!(
        text.lines()
            .nth(1)
            .unwrap()
            .split_whitespace()
            .collect::<Vec<_>>(),
        vec![
            "aaaaaaaaaaaa",
            "2",
            "running",
            "100",
            "1000",
            "4",
            "1024",
            "2048",
            "900",
            "1024",
            "0",
            "123",
            "demo"
        ]
    );
}

#[tokio::test]
async fn stats_stream_refreshes_until_rpc_failure() {
    let (out, calls) = run_stats(
        &[],
        vec![
            sample(100, 1000),
            sample(150, 1200),
            sample(170, 1400),
            Err(Status::unavailable("disconnected")),
        ],
    )
    .await;
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(calls.len(), 4);
    let text = String::from_utf8(out.stdout).unwrap();
    assert_eq!(text.matches("\x1b[H\x1b[J").count(), 2);
    assert!(text.contains("100.00") && text.contains("40.00"));
    assert!(String::from_utf8(out.stderr)
        .unwrap()
        .contains("disconnected"));
}

#[tokio::test]
async fn stats_empty_results_and_errors() {
    let (out, calls) = run_stats(
        &["--no-stream"],
        vec![Ok(StatsResponse::default()), Ok(StatsResponse::default())],
    )
    .await;
    assert!(out.status.success());
    assert_eq!(calls.len(), 2);
    assert!(String::from_utf8(out.stdout).unwrap().contains("CONTAINER"));
    for replies in [
        vec![Ok(StatsResponse {
            cc: 42,
            errmsg: "stats failed".into(),
            ..Default::default()
        })],
        vec![sample(100, 1000), Err(Status::unavailable("stats failed"))],
    ] {
        let (out, _) = run_stats(&["--no-stream"], replies).await;
        assert_eq!(out.status.code(), Some(1));
        assert!(out.stdout.is_empty());
        assert!(String::from_utf8(out.stderr)
            .unwrap()
            .contains("stats failed"));
    }
}

#[tokio::test]
async fn stats_boolean_overrides_and_argument_validation() {
    let (out, calls) = run_stats(
        &[
            "-a",
            "--all=false",
            "--no-stream=false",
            "--original=true",
            "--",
            "-name",
        ],
        vec![sample(1, 2)],
    )
    .await;
    assert!(out.status.success());
    assert_eq!(
        calls,
        vec![StatsRequest {
            all: false,
            containers: vec!["-name".into()]
        }]
    );
    for args in [
        vec!["--all=bad"],
        vec!["--no-stream=bad"],
        vec!["--format", "x"],
        vec!["-H"],
    ] {
        let (out, calls) = run_stats(&args, vec![]).await;
        assert_eq!(out.status.code(), Some(125));
        assert!(calls.is_empty());
    }
    let (out, calls) = run_stats(&["--help"], vec![]).await;
    assert!(out.status.success());
    assert!(calls.is_empty());
}

#[tokio::test]
async fn stats_matches_previous_samples_by_id_and_new_containers_start_at_zero() {
    let mut first = sample(100, 1000).unwrap();
    let mut second = sample(150, 1200).unwrap();
    let new = proto::ContainerInfo {
        id: "b".repeat(64),
        ..Default::default()
    };
    first.containers.push(proto::ContainerInfo {
        id: "gone".into(),
        ..Default::default()
    });
    second.containers.insert(0, new);
    let (out, _) = run_stats(&["--no-stream"], vec![Ok(first), Ok(second)]).await;
    assert!(out.status.success());
    let text = String::from_utf8(out.stdout).unwrap();
    let rows: Vec<_> = text.lines().collect();
    assert!(rows[1].contains("bbbbbbbbbbbb     0.00"));
    assert!(rows[2].contains("aaaaaaaaaaaa     100.00"));
    assert!(!text.contains("gone"));
}
