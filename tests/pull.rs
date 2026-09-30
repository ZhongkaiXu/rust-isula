use std::process::{Command, Output};
use std::time::Duration;

use risula::grpc::images_proto::{
    self, ListImagesRequest, ListImagesResponse, LoadImageRequest, LoadImageResponse,
    PullImageRequest, PullImageResponse,
};
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{Request, Response, Status};

struct FakePull {
    expected_name: String,
    replies: Vec<PullImageResponse>,
    fail: bool,
    delay: Duration,
}

#[tonic::async_trait]
impl images_proto::images_service_server::ImagesService for FakePull {
    async fn list(
        &self,
        _request: Request<ListImagesRequest>,
    ) -> Result<Response<ListImagesResponse>, Status> {
        Err(Status::unimplemented("list"))
    }

    async fn load(
        &self,
        _request: Request<LoadImageRequest>,
    ) -> Result<Response<LoadImageResponse>, Status> {
        Err(Status::unimplemented("load"))
    }

    type PullImageStream =
        tokio_stream::Iter<std::vec::IntoIter<Result<PullImageResponse, Status>>>;

    async fn pull_image(
        &self,
        request: Request<PullImageRequest>,
    ) -> Result<Response<Self::PullImageStream>, Status> {
        let request = request.into_inner();
        if request.image.as_ref().map(|image| image.image.as_str())
            != Some(self.expected_name.as_str())
            || request.auth.is_some()
            || request.is_progress_visible
        {
            return Err(Status::invalid_argument("wrong pull request"));
        }

        tokio::time::sleep(self.delay).await;
        let mut replies: Vec<_> = self.replies.iter().cloned().map(Ok).collect();
        if self.fail {
            replies.push(Err(Status::internal("registry unavailable")));
        }
        Ok(Response::new(tokio_stream::iter(replies)))
    }
}

async fn run_pull(replies: Vec<PullImageResponse>, fail: bool, delay: Duration) -> Output {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("isulad.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(
                images_proto::images_service_server::ImagesServiceServer::new(FakePull {
                    expected_name: "example:test".into(),
                    replies,
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
        .args(["pull", "-H", &host, "example:test"])
        .output()
        .unwrap();
    server.abort();
    output
}

#[tokio::test(flavor = "multi_thread")]
async fn pull_reads_all_frames_and_reports_final_ref() {
    let replies = vec![
        PullImageResponse {
            progress_data: br#"{"progresses":[{"id":"layer","current":1,"total":2}]}"#.to_vec(),
            ..Default::default()
        },
        PullImageResponse {
            image_ref: "example:test".into(),
            ..Default::default()
        },
    ];
    let output = run_pull(replies, false, Duration::ZERO).await;
    assert!(output.status.success());
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "Image \"example:test\" pulling\nImage \"example:test\" pulled\n"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn pull_reports_stream_error() {
    let reply = PullImageResponse {
        image_ref: "example:test".into(),
        ..Default::default()
    };
    let output = run_pull(vec![reply], true, Duration::ZERO).await;
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("registry unavailable"));
    assert!(!stderr.contains("pulled"));
}

#[tokio::test(flavor = "multi_thread")]
async fn pull_rejects_stream_without_image_ref() {
    let output = run_pull(vec![PullImageResponse::default()], false, Duration::ZERO).await;
    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("without an image reference"));
}

#[tokio::test(flavor = "multi_thread")]
async fn pull_can_run_longer_than_short_rpc_timeout() {
    let reply = PullImageResponse {
        image_ref: "example:test".into(),
        ..Default::default()
    };
    let output = run_pull(vec![reply], false, Duration::from_secs(6)).await;
    assert!(output.status.success());
}
