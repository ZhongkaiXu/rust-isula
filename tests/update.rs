use risula::grpc::proto::{self, ExecRequest};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, UnixListener};
use tokio_stream::wrappers::{TcpListenerStream, UnixListenerStream};
use tonic::{Request, Response, Status};
struct FakeUpdate {
    calls: Arc<Mutex<Vec<proto::UpdateRequest>>>,
}
#[tonic::async_trait]
impl proto::container_service_server::ContainerService for FakeUpdate {
    async fn update(
        &self,
        request: Request<proto::UpdateRequest>,
    ) -> Result<Response<proto::UpdateResponse>, Status> {
        let request = request.into_inner();
        self.calls.lock().unwrap().push(request.clone());
        if request.id == "rpc-error" {
            return Err(Status::unavailable("transport failed"));
        }
        if request.id == "slow" {
            tokio::time::sleep(std::time::Duration::from_secs(6)).await;
        }
        Ok(Response::new(proto::UpdateResponse {
            id: "resolved-id".into(),
            cc: if request.id == "failure" { 2 } else { 0 },
            errmsg: "server message".into(),
        }))
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

async fn run_at(args: &[&str], tcp: bool) -> (Output, Vec<proto::UpdateRequest>) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("isulad.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let tcp_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = tcp_listener.local_addr().unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let server_calls = calls.clone();
    let server = tokio::spawn(async move {
        let service = proto::container_service_server::ContainerServiceServer::new(FakeUpdate {
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
        command.args(["update", "-H", &host]).args(&args);
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
async fn update_packs_resources_and_prints_original_names_on_both_transports() {
    for tcp in [false, true] {
        let (out, calls) = run_at(
            &[
                "--cpu-shares",
                "0x200",
                "--cpu-period",
                "100000",
                "--cpu-quota",
                "50000",
                "--cpuset-cpus",
                "0-1",
                "--cpuset-mems",
                "0",
                "-m",
                "64m",
                "--memory-swap",
                "-1",
                "--memory-reservation",
                "32m",
                "--kernel-memory",
                "8m",
                "--blkio-weight",
                "500",
                "--cpu-rt-period",
                "1000000",
                "--cpu-rt-runtime",
                "500000",
                "--restart",
                "on-failure:3",
                "demo",
            ],
            tcp,
        )
        .await;
        assert!(out.status.success(), "{out:?}");
        assert_eq!(out.stdout, b"demo\n");
        assert!(out.stderr.is_empty());
        assert_eq!(calls.len(), 1);
        assert_eq!(calls[0].id, "demo");
        let host: serde_json::Value = serde_json::from_str(&calls[0].hostconfig).unwrap();
        assert_eq!(
            host,
            serde_json::json!({"CPUShares":512,"CPUPeriod":100000,"CPUQuota":50000,"CpusetCpus":"0-1","CpusetMems":"0","Memory":67108864,"MemorySwap":-1,"MemoryReservation":33554432,"KernelMemory":8388608,"BlkioWeight":500,"CPURealtimePeriod":1000000,"CPURealtimeRuntime":500000,"RestartPolicy":{"Name":"on-failure","MaximumRetryCount":3}})
        );
    }
}
#[tokio::test]
async fn update_omits_defaults_and_uses_final_values() {
    for (args, expected) in [
        (vec!["demo"], serde_json::json!({})),
        (
            vec!["--cpus", "1.25", "demo"],
            serde_json::json!({"NanoCpus":1250000000}),
        ),
        (
            vec![
                "--cpu-shares",
                "512",
                "--cpu-shares",
                "0",
                "--cpuset-cpus",
                "0",
                "--cpuset-cpus=",
                "--restart",
                "bad:x",
                "--restart",
                "no",
                "demo",
            ],
            serde_json::json!({"RestartPolicy":{"Name":"no"}}),
        ),
        (
            vec!["--restart=", "demo"],
            serde_json::json!({"RestartPolicy":{"Name":""}}),
        ),
    ] {
        let (out, calls) = run_at(&args, false).await;
        assert!(out.status.success(), "{out:?}");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&calls[0].hostconfig).unwrap(),
            expected
        );
    }
}
#[tokio::test]
async fn update_continues_after_errors_and_reuses_configuration() {
    let (out, calls) = run_at(
        &[
            "--cpu-shares",
            "512",
            "failure",
            "ok",
            "",
            "rpc-error",
            "last",
        ],
        false,
    )
    .await;
    assert_eq!(out.status.code(), Some(1));
    assert_eq!(out.stdout, b"ok\nlast\n");
    assert_eq!(
        calls.iter().map(|r| r.id.as_str()).collect::<Vec<_>>(),
        ["failure", "ok", "rpc-error", "last"]
    );
    assert!(calls.iter().all(|r| r.hostconfig == calls[0].hostconfig));
}
#[tokio::test]
async fn update_invalid_arguments_never_call_daemon() {
    for (args, code) in [
        (vec![], 125),
        (vec!["--memory", "-1", "demo"], 125),
        (vec!["--cpus", "-1", "demo"], 125),
        (vec!["--blkio-weight", "65536", "demo"], 125),
        (vec!["--cpu-shares", "bad", "demo"], 125),
        (vec!["--restart", "on-failure:bad", "demo"], 1),
        (vec!["--memory", "bad", "--memory", "8m", "demo"], 125),
        (vec!["demo"; 1000], 1),
        (vec!["--unknown"], 125),
    ] {
        let (out, calls) = run_at(&args, false).await;
        assert_eq!(out.status.code(), Some(code), "{args:?} {out:?}");
        assert!(calls.is_empty());
    }
    let out = Command::new(env!("CARGO_BIN_EXE_risula"))
        .args(["update", "demo"])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
}
#[tokio::test]
async fn update_waits_for_slow_daemon() {
    let (out, _) = run_at(&["--restart", "no", "slow"], true).await;
    assert!(out.status.success(), "{out:?}");
}

#[tokio::test]
async fn update_matches_native_request_fixtures() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("fixtures/update-native.json")).unwrap();
    for case in cases.as_array().unwrap() {
        let args: Vec<&str> = case["args"]
            .as_array()
            .unwrap()
            .iter()
            .map(|v| v.as_str().unwrap())
            .collect();
        let (out, calls) = run_at(&args, false).await;
        assert_eq!(
            out.status.code(),
            Some(case["code"].as_i64().unwrap() as i32),
            "{args:?}: {out:?}"
        );
        assert_eq!(
            String::from_utf8(out.stdout).unwrap(),
            case["stdout"].as_str().unwrap(),
            "{args:?}"
        );
        let requests: Vec<_> = calls.iter().map(|r| serde_json::json!({"id":r.id,"host":serde_json::from_str::<serde_json::Value>(&r.hostconfig).unwrap()})).collect();
        assert_eq!(serde_json::json!(requests), case["requests"], "{args:?}");
    }
}
