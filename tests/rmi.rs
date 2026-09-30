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

struct FakeDelete {
    calls: Arc<Mutex<Vec<DeleteImageRequest>>>,
}

#[tonic::async_trait]
impl images_proto::images_service_server::ImagesService for FakeDelete {
    async fn list(
        &self,
        _request: Request<ListImagesRequest>,
    ) -> Result<Response<ListImagesResponse>, Status> {
        Err(Status::unimplemented("list"))
    }

    async fn delete(
        &self,
        request: Request<DeleteImageRequest>,
    ) -> Result<Response<DeleteImageResponse>, Status> {
        let request = request.into_inner();
        self.calls.lock().unwrap().push(request.clone());
        if request.name == "missing" {
            return Ok(Response::new(DeleteImageResponse {
                cc: 42,
                errmsg: "image not found".into(),
                ..Default::default()
            }));
        }
        if request.name == "unavailable" {
            return Err(Status::unavailable("image service unavailable"));
        }
        if request.name == "slow" {
            tokio::time::sleep(std::time::Duration::from_secs(6)).await;
        }
        Ok(Response::new(DeleteImageResponse::default()))
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

    type PullImageStream =
        tokio_stream::wrappers::ReceiverStream<Result<PullImageResponse, Status>>;

    async fn pull_image(
        &self,
        _request: Request<PullImageRequest>,
    ) -> Result<Response<Self::PullImageStream>, Status> {
        Err(Status::unimplemented("pull"))
    }
}

async fn run_rmi(args: &[&str]) -> (Output, Vec<DeleteImageRequest>) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("isulad.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let server_calls = calls.clone();
    let server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(
                images_proto::images_service_server::ImagesServiceServer::new(FakeDelete {
                    calls: server_calls,
                }),
            )
            .serve_with_incoming(UnixListenerStream::new(listener))
            .await
            .unwrap();
    });
    let host = format!("unix://{}", socket.display());
    let output = Command::new(env!("CARGO_BIN_EXE_risula"))
        .args(["rmi", "-H", &host])
        .args(args)
        .output()
        .unwrap();
    server.abort();
    let seen = calls.lock().unwrap().clone();
    (output, seen)
}

#[tokio::test(flavor = "multi_thread")]
async fn rmi_sends_force_and_removes_multiple_images() {
    let (output, calls) = run_rmi(&["-f", "first:tag", "second:tag"]).await;
    assert!(output.status.success());
    assert_eq!(
        calls,
        vec![
            DeleteImageRequest {
                name: "first:tag".into(),
                force: true,
            },
            DeleteImageRequest {
                name: "second:tag".into(),
                force: true,
            },
        ]
    );
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "Image \"first:tag\" removed\nImage \"second:tag\" removed\n"
    );
    assert!(output.stderr.is_empty());
}

#[tokio::test(flavor = "multi_thread")]
async fn rmi_continues_after_server_error_and_reserved_name() {
    let (output, calls) = run_rmi(&["missing", "none", "good:tag"]).await;
    assert!(!output.status.success());
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0].name, "missing");
    assert_eq!(calls[1].name, "good:tag");
    assert!(!calls[0].force);
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "Image \"good:tag\" removed\n"
    );
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("image not found"));
    assert!(stderr.contains("reserved image name"));
}

#[tokio::test(flavor = "multi_thread")]
async fn rmi_continues_after_grpc_error() {
    let (output, calls) = run_rmi(&["unavailable", "good:tag"]).await;
    assert!(!output.status.success());
    assert_eq!(calls.len(), 2);
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("image service unavailable"));
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "Image \"good:tag\" removed\n"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn rmi_can_take_longer_than_short_rpc_timeout() {
    let (output, calls) = run_rmi(&["slow"]).await;
    assert!(output.status.success());
    assert_eq!(calls.len(), 1);
}
