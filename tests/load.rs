use std::fs;
use std::process::{Command, Output};

use risula::grpc::images_proto::{
    self, DeleteImageRequest, DeleteImageResponse, ImportRequest, ImportResponse,
    ListImagesRequest, ListImagesResponse, LoadImageRequest, LoadImageResponse, PullImageRequest,
    PullImageResponse,
};
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{Request, Response, Status};

struct FakeLoad {
    expected: LoadImageRequest,
    reply: LoadImageResponse,
}

#[tonic::async_trait]
impl images_proto::images_service_server::ImagesService for FakeLoad {
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
        request: Request<LoadImageRequest>,
    ) -> Result<Response<LoadImageResponse>, Status> {
        if request.into_inner() != self.expected {
            return Err(Status::invalid_argument("wrong load request"));
        }
        Ok(Response::new(self.reply.clone()))
    }

    async fn import(
        &self,
        _request: Request<ImportRequest>,
    ) -> Result<Response<ImportResponse>, Status> {
        Err(Status::unimplemented("import"))
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

async fn run_load(tag: Option<&str>, reply: LoadImageResponse) -> Output {
    let directory = tempfile::tempdir().unwrap();
    let archive = directory.path().join("image.tar");
    fs::write(&archive, b"archive fixture").unwrap();

    let socket = directory.path().join("isulad.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let expected = LoadImageRequest {
        file: archive.to_str().unwrap().to_string(),
        r#type: "oci".into(),
        tag: tag.unwrap_or_default().into(),
    };
    let server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(
                images_proto::images_service_server::ImagesServiceServer::new(FakeLoad {
                    expected,
                    reply,
                }),
            )
            .serve_with_incoming(UnixListenerStream::new(listener))
            .await
            .unwrap();
    });

    let host = format!("unix://{}", socket.display());
    let mut command = Command::new(env!("CARGO_BIN_EXE_risula"));
    command
        .current_dir(directory.path())
        .args(["load", "-H", &host, "-i", "image.tar"]);
    if let Some(tag) = tag {
        command.args(["--tag", tag]);
    }
    let output = command.output().unwrap();
    server.abort();
    output
}

#[tokio::test(flavor = "multi_thread")]
async fn load_sends_absolute_path_type_and_tag() {
    let output = run_load(Some("example:test"), LoadImageResponse::default()).await;
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.starts_with("Load image from \"/"));
    assert!(stdout.ends_with("/image.tar\" success\n"));
}

#[tokio::test(flavor = "multi_thread")]
async fn load_omits_tag_when_unspecified() {
    let output = run_load(None, LoadImageResponse::default()).await;
    assert!(output.status.success());
}

#[tokio::test(flavor = "multi_thread")]
async fn load_reports_daemon_error() {
    let reply = LoadImageResponse {
        cc: 42,
        errmsg: "invalid archive".into(),
    };
    let output = run_load(None, reply).await;
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("42"));
    assert!(stderr.contains("invalid archive"));
}

#[test]
fn load_rejects_missing_file_before_connecting() {
    let directory = tempfile::tempdir().unwrap();
    let socket = format!("unix://{}", directory.path().join("isulad.sock").display());
    let missing = directory.path().join("missing.tar");
    let output = Command::new(env!("CARGO_BIN_EXE_risula"))
        .args(["load", "-H", &socket, "-i", missing.to_str().unwrap()])
        .output()
        .unwrap();
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("cannot read"));
    assert!(!stderr.contains("cannot connect"));
}
