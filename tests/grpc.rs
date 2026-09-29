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

#[tokio::test(flavor = "multi_thread")]
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

#[tokio::test(flavor = "multi_thread")]
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
