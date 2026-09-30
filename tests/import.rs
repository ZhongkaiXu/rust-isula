use std::process::{Command, Output};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use risula::grpc::images_proto::{
    self, DeleteImageRequest, DeleteImageResponse, ImportRequest, ImportResponse,
    ListImagesRequest, ListImagesResponse, LoadImageRequest, LoadImageResponse, PullImageRequest,
    PullImageResponse,
};
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{Request, Response, Status};

struct FakeImport {
    calls: Arc<Mutex<Vec<ImportRequest>>>,
    reply: ImportResponse,
    fail: bool,
    delay: Duration,
}

#[tonic::async_trait]
impl images_proto::images_service_server::ImagesService for FakeImport {
    async fn list(
        &self,
        _request: Request<ListImagesRequest>,
    ) -> Result<Response<ListImagesResponse>, Status> {
        Err(Status::unimplemented("list"))
    }

    async fn delete(
        &self,
        _request: Request<DeleteImageRequest>,
    ) -> Result<Response<DeleteImageResponse>, Status> {
        Err(Status::unimplemented("delete"))
    }

    async fn load(
        &self,
        _request: Request<LoadImageRequest>,
    ) -> Result<Response<LoadImageResponse>, Status> {
        Err(Status::unimplemented("load"))
    }

    async fn import(
        &self,
        request: Request<ImportRequest>,
    ) -> Result<Response<ImportResponse>, Status> {
        self.calls.lock().unwrap().push(request.into_inner());
        tokio::time::sleep(self.delay).await;
        if self.fail {
            return Err(Status::unavailable("image service unavailable"));
        }
        Ok(Response::new(self.reply.clone()))
    }

    type PullImageStream =
        tokio_stream::wrappers::ReceiverStream<Result<PullImageResponse, Status>>;

    async fn pull_image(
        &self,
        _request: Request<PullImageRequest>,
    ) -> Result<Response<Self::PullImageStream>, Status> {
        Err(Status::unimplemented("pull"))
    }
}

async fn run_import(
    tag: &str,
    reply: ImportResponse,
    fail: bool,
    delay: Duration,
) -> (Output, Vec<ImportRequest>, String) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("isulad.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let server_calls = calls.clone();
    let server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(
                images_proto::images_service_server::ImagesServiceServer::new(FakeImport {
                    calls: server_calls,
                    reply,
                    fail,
                    delay,
                }),
            )
            .serve_with_incoming(UnixListenerStream::new(listener))
            .await
            .unwrap();
    });
    let host = format!("unix://{}", socket.display());
    let output = Command::new(env!("CARGO_BIN_EXE_risula"))
        .current_dir(directory.path())
        .args(["import", "-H", &host, "rootfs.tar", tag])
        .output()
        .unwrap();
    server.abort();
    let seen = calls.lock().unwrap().clone();
    let path = directory.path().join("rootfs.tar").display().to_string();
    (output, seen, path)
}

#[tokio::test(flavor = "multi_thread")]
async fn import_sends_absolute_path_and_tag_and_prints_id() {
    let reply = ImportResponse {
        id: "abc123".into(),
        ..Default::default()
    };
    let (output, calls, path) = run_import("example:test", reply, false, Duration::ZERO).await;
    assert!(output.status.success());
    assert_eq!(
        calls,
        vec![ImportRequest {
            file: path,
            tag: "example:test".into()
        }]
    );
    assert_eq!(String::from_utf8(output.stdout).unwrap(), "sha256:abc123\n");
    assert!(output.stderr.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn import_reports_daemon_error() {
    let reply = ImportResponse {
        cc: 42,
        errmsg: "invalid archive".into(),
        ..Default::default()
    };
    let (output, calls, _) = run_import("example:test", reply, false, Duration::ZERO).await;
    assert!(!output.status.success());
    assert_eq!(calls.len(), 1);
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("42"));
    assert!(stderr.contains("invalid archive"));
}

#[tokio::test(flavor = "multi_thread")]
async fn import_reports_grpc_error() {
    let (output, calls, _) = run_import(
        "example:test",
        ImportResponse::default(),
        true,
        Duration::ZERO,
    )
    .await;
    assert!(!output.status.success());
    assert_eq!(calls.len(), 1);
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("image service unavailable"));
}

#[tokio::test(flavor = "multi_thread")]
async fn import_rejects_digest_as_tag_before_connecting() {
    let tag = format!("sha256:{}", "a".repeat(64));
    let (output, calls, _) =
        run_import(&tag, ImportResponse::default(), false, Duration::ZERO).await;
    assert!(!output.status.success());
    assert!(calls.is_empty());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("not a valid image name"));
}

#[tokio::test(flavor = "multi_thread")]
async fn import_can_run_longer_than_short_rpc_timeout() {
    let (output, calls, _) = run_import(
        "example:test",
        ImportResponse::default(),
        false,
        Duration::from_secs(6),
    )
    .await;
    assert!(output.status.success());
    assert_eq!(calls.len(), 1);
    assert!(output.stdout.is_empty());
}
