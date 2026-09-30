use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

use risula::grpc::images_proto::{
    self, DeleteImageRequest, DeleteImageResponse, ImportRequest, ImportResponse,
    ListImagesRequest, ListImagesResponse, LoadImageRequest, LoadImageResponse, LoginRequest,
    LoginResponse, PullImageRequest, PullImageResponse, TagImageRequest, TagImageResponse,
};
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{Request, Response, Status};

struct FakeTag {
    calls: Arc<Mutex<Vec<TagImageRequest>>>,
    reply: TagImageResponse,
    fail: bool,
}

#[tonic::async_trait]
impl images_proto::images_service_server::ImagesService for FakeTag {
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

    async fn tag(
        &self,
        request: Request<TagImageRequest>,
    ) -> Result<Response<TagImageResponse>, Status> {
        self.calls.lock().unwrap().push(request.into_inner());
        if self.fail {
            return Err(Status::unavailable("image service unavailable"));
        }
        Ok(Response::new(self.reply.clone()))
    }

    async fn load(
        &self,
        _request: Request<LoadImageRequest>,
    ) -> Result<Response<LoadImageResponse>, Status> {
        Err(Status::unimplemented("load"))
    }

    async fn import(
        &self,
        _request: Request<ImportRequest>,
    ) -> Result<Response<ImportResponse>, Status> {
        Err(Status::unimplemented("import"))
    }

    async fn login(
        &self,
        _request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        Err(Status::unimplemented("login"))
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

async fn run_tag(
    destination: &str,
    reply: TagImageResponse,
    fail: bool,
) -> (Output, Vec<TagImageRequest>) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("isulad.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let server_calls = calls.clone();
    let server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(
                images_proto::images_service_server::ImagesServiceServer::new(FakeTag {
                    calls: server_calls,
                    reply,
                    fail,
                }),
            )
            .serve_with_incoming(UnixListenerStream::new(listener))
            .await
            .unwrap();
    });
    let host = format!("unix://{}", socket.display());
    let output = Command::new(env!("CARGO_BIN_EXE_risula"))
        .args(["tag", "-H", &host, "source:old", destination])
        .output()
        .unwrap();
    server.abort();
    let seen = calls.lock().unwrap().clone();
    (output, seen)
}

#[tokio::test(flavor = "multi_thread")]
async fn tag_sends_source_and_destination_without_output() {
    let (output, calls) = run_tag("target:new", TagImageResponse::default(), false).await;
    assert!(output.status.success());
    assert_eq!(
        calls,
        vec![TagImageRequest {
            src_name: "source:old".into(),
            dest_name: "target:new".into(),
        }]
    );
    assert!(output.stdout.is_empty());
    assert!(output.stderr.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn tag_reports_daemon_error() {
    let reply = TagImageResponse {
        cc: 42,
        errmsg: "source image not found".into(),
    };
    let (output, calls) = run_tag("target:new", reply, false).await;
    assert!(!output.status.success());
    assert_eq!(calls.len(), 1);
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("42"));
    assert!(stderr.contains("source image not found"));
}

#[tokio::test(flavor = "multi_thread")]
async fn tag_reports_grpc_error() {
    let (output, calls) = run_tag("target:new", TagImageResponse::default(), true).await;
    assert!(!output.status.success());
    assert_eq!(calls.len(), 1);
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("image service unavailable"));
}

#[tokio::test(flavor = "multi_thread")]
async fn tag_rejects_digest_destination_before_connecting() {
    let destination = format!("sha256:{}", "a".repeat(64));
    let (output, calls) = run_tag(&destination, TagImageResponse::default(), false).await;
    assert!(!output.status.success());
    assert!(calls.is_empty());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("not a valid tag"));
}
