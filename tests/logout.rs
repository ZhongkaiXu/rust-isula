use std::process::{Command, Output};
use std::sync::{Arc, Mutex};

use risula::grpc::images_proto::{
    self, DeleteImageRequest, DeleteImageResponse, ImportRequest, ImportResponse,
    ListImagesRequest, ListImagesResponse, LoadImageRequest, LoadImageResponse, LoginRequest,
    LoginResponse, LogoutRequest, LogoutResponse, PullImageRequest, PullImageResponse,
    SearchRequest, SearchResponse, TagImageRequest, TagImageResponse,
};
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{Request, Response, Status};

struct FakeLogout {
    calls: Arc<Mutex<Vec<LogoutRequest>>>,
    reply: LogoutResponse,
    fail: bool,
}

#[tonic::async_trait]
impl images_proto::images_service_server::ImagesService for FakeLogout {
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
        _request: Request<TagImageRequest>,
    ) -> Result<Response<TagImageResponse>, Status> {
        Err(Status::unimplemented("tag"))
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

    async fn logout(
        &self,
        request: Request<LogoutRequest>,
    ) -> Result<Response<LogoutResponse>, Status> {
        self.calls.lock().unwrap().push(request.into_inner());
        if self.fail {
            return Err(Status::unavailable("image service unavailable"));
        }
        Ok(Response::new(self.reply.clone()))
    }

    async fn search(
        &self,
        _request: Request<SearchRequest>,
    ) -> Result<Response<SearchResponse>, Status> {
        Err(Status::unimplemented("search"))
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

async fn run_logout(reply: LogoutResponse, fail: bool) -> (Output, Vec<LogoutRequest>) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("isulad.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let server_calls = calls.clone();
    let server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(
                images_proto::images_service_server::ImagesServiceServer::new(FakeLogout {
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
        .args(["logout", "-H", &host, "registry.test"])
        .output()
        .unwrap();
    server.abort();
    let seen = calls.lock().unwrap().clone();
    (output, seen)
}

#[tokio::test(flavor = "multi_thread")]
async fn logout_sends_server_and_oci_type() {
    let (output, calls) = run_logout(LogoutResponse::default(), false).await;
    assert!(output.status.success());
    assert_eq!(
        calls,
        vec![LogoutRequest {
            server: "registry.test".into(),
            r#type: "oci".into(),
        }]
    );
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "Logout Succeeded\n"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn logout_reports_daemon_error() {
    let reply = LogoutResponse {
        cc: 42,
        errmsg: "cannot remove credentials".into(),
    };
    let (output, calls) = run_logout(reply, false).await;
    assert!(!output.status.success());
    assert_eq!(calls.len(), 1);
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("42"));
    assert!(stderr.contains("cannot remove credentials"));
    assert!(!stderr.contains("Logout Succeeded"));
}

#[tokio::test(flavor = "multi_thread")]
async fn logout_reports_grpc_error() {
    let (output, calls) = run_logout(LogoutResponse::default(), true).await;
    assert!(!output.status.success());
    assert_eq!(calls.len(), 1);
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("image service unavailable"));
}
