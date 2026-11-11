use risula::grpc::{self, proto};
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{Request, Response, Status};

#[derive(Default)]
struct FakeDaemon {
    version_reply: proto::VersionResponse,
    info_reply: proto::InfoResponse,
}

#[tonic::async_trait]
impl proto::container_service_server::ContainerService for FakeDaemon {
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

    async fn stop(
        &self,
        _request: Request<proto::StopRequest>,
    ) -> Result<Response<proto::StopResponse>, Status> {
        Err(Status::unimplemented("stop"))
    }

    async fn version(
        &self,
        _request: Request<proto::VersionRequest>,
    ) -> Result<Response<proto::VersionResponse>, Status> {
        Ok(Response::new(self.version_reply.clone()))
    }

    async fn info(
        &self,
        _request: Request<proto::InfoRequest>,
    ) -> Result<Response<proto::InfoResponse>, Status> {
        Ok(Response::new(self.info_reply.clone()))
    }
}

async fn start_fake_daemon(
    daemon: FakeDaemon,
) -> (tempfile::TempDir, String, tokio::task::JoinHandle<()>) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("isulad.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(proto::container_service_server::ContainerServiceServer::new(daemon))
            .serve_with_incoming(UnixListenerStream::new(listener))
            .await
            .unwrap();
    });
    (directory, format!("unix://{}", socket.display()), server)
}

#[tokio::test]
async fn unix_rpc_client_does_not_start_worker_threads() {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("isulad.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let host = format!("unix://{}", socket.display());
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_risula"))
        .env("TOKIO_WORKER_THREADS", "8")
        .args(["version", "-H", &host])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .spawn()
        .unwrap();

    // Hold the connection open without replying, so the CLI stays alive for inspection.
    let connection =
        tokio::time::timeout(std::time::Duration::from_secs(3), listener.accept()).await;
    let status = std::fs::read_to_string(format!("/proc/{}/status", child.id()));
    let killed = child.kill();
    let reaped = child.wait();

    killed.unwrap();
    reaped.unwrap();
    let _connection = connection.unwrap().unwrap();
    let status = status.unwrap();
    let threads = status
        .lines()
        .find_map(|line| line.strip_prefix("Threads:"))
        .unwrap()
        .trim();
    assert_eq!(
        threads, "1",
        "RPC client unexpectedly created worker threads"
    );
}

#[tokio::test]
async fn version_reads_server_reply_over_unix_socket() {
    let reply = proto::VersionResponse {
        version: "2.1.5".into(),
        git_commit: "abc123".into(),
        build_time: "2025-01-08".into(),
        root_path: "/var/lib/isulad".into(),
        cc: 0,
        errmsg: String::new(),
    };
    let (_directory, host, server) = start_fake_daemon(FakeDaemon {
        version_reply: reply,
        ..Default::default()
    })
    .await;
    let response = grpc::version(&host).await.unwrap();
    server.abort();
    assert_eq!(response.version, "2.1.5");
    assert_eq!(response.git_commit, "abc123");
    assert_eq!(response.root_path, "/var/lib/isulad");
}

#[tokio::test]
async fn version_reports_server_error() {
    let reply = proto::VersionResponse {
        cc: 42,
        errmsg: "daemon rejected request".into(),
        ..Default::default()
    };
    let (_directory, host, server) = start_fake_daemon(FakeDaemon {
        version_reply: reply,
        ..Default::default()
    })
    .await;
    let error = grpc::version(&host).await.unwrap_err();
    server.abort();
    assert!(error.contains("42"));
    assert!(error.contains("daemon rejected request"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn version_command_reaches_selected_daemon() {
    let reply = proto::VersionResponse {
        version: "2.1.5".into(),
        git_commit: "abc123".into(),
        build_time: "2025-01-08".into(),
        ..Default::default()
    };
    let (_directory, host, server) = start_fake_daemon(FakeDaemon {
        version_reply: reply,
        ..Default::default()
    })
    .await;
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_risula"))
        .args(["version", "-H", &host])
        .output()
        .unwrap();
    server.abort();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Server:\n  Version:\t2.1.5"));
    assert!(stdout.contains("Git commit:\tabc123"));
}

#[tokio::test]
async fn info_reads_server_reply_over_unix_socket() {
    let reply = proto::InfoResponse {
        containers_num: 2,
        images_num: 7,
        architecture: "aarch64".into(),
        ..Default::default()
    };
    let (_directory, host, server) = start_fake_daemon(FakeDaemon {
        info_reply: reply,
        ..Default::default()
    })
    .await;
    let response = grpc::info(&host).await.unwrap();
    server.abort();
    assert_eq!(response.containers_num, 2);
    assert_eq!(response.images_num, 7);
    assert_eq!(response.architecture, "aarch64");
}

#[tokio::test]
async fn info_reports_server_error() {
    let reply = proto::InfoResponse {
        cc: 42,
        errmsg: "daemon rejected info".into(),
        ..Default::default()
    };
    let (_directory, host, server) = start_fake_daemon(FakeDaemon {
        info_reply: reply,
        ..Default::default()
    })
    .await;
    let error = grpc::info(&host).await.unwrap_err();
    server.abort();
    assert!(error.contains("42"));
    assert!(error.contains("daemon rejected info"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn info_command_displays_server_fields() {
    let reply = proto::InfoResponse {
        containers_num: 2,
        c_stopped: 2,
        images_num: 7,
        version: "2.1.5".into(),
        driver_name: "overlay".into(),
        driver_status: "Backing Filesystem: extfs\nSupports d_type: true\n".into(),
        architecture: "aarch64".into(),
        total_mem: 1006,
        ..Default::default()
    };
    let (_directory, host, server) = start_fake_daemon(FakeDaemon {
        info_reply: reply,
        ..Default::default()
    })
    .await;
    let output = std::process::Command::new(env!("CARGO_BIN_EXE_risula"))
        .args(["info", "-H", &host])
        .output()
        .unwrap();
    server.abort();
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("Containers: 2\n Running: 0\n Paused: 0\n Stopped: 2\n"));
    assert!(stdout
        .contains("Storage Driver: overlay\n Backing Filesystem: extfs\n Supports d_type: true\n"));
    assert!(stdout.contains("Architecture: aarch64\nCPUs: 0\nTotal Memory: 1006 GB\n"));
}
