use std::collections::HashMap;
use std::process::{Command, Output};

use risula::grpc::images_proto::{
    self, DeleteImageRequest, DeleteImageResponse, Descriptor, Image, ImportRequest,
    ImportResponse, ListImagesRequest, ListImagesResponse, LoadImageRequest, LoadImageResponse,
    LoginRequest, LoginResponse, PullImageRequest, PullImageResponse, TagImageRequest,
    TagImageResponse,
};
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{Request, Response, Status};

struct FakeImages {
    expected_filters: HashMap<String, String>,
    reply: ListImagesResponse,
}

#[tonic::async_trait]
impl images_proto::images_service_server::ImagesService for FakeImages {
    async fn list(
        &self,
        request: Request<ListImagesRequest>,
    ) -> Result<Response<ListImagesResponse>, Status> {
        if request.into_inner().filters != self.expected_filters {
            return Err(Status::invalid_argument("wrong filters"));
        }
        Ok(Response::new(self.reply.clone()))
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

    type PullImageStream =
        tokio_stream::wrappers::ReceiverStream<Result<PullImageResponse, Status>>;

    async fn pull_image(
        &self,
        _request: Request<PullImageRequest>,
    ) -> Result<Response<Self::PullImageStream>, Status> {
        Err(Status::unimplemented("pull"))
    }
}

async fn run_images(
    reply: ListImagesResponse,
    expected_filters: HashMap<String, String>,
    args: &[&str],
) -> Output {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("isulad.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(
                images_proto::images_service_server::ImagesServiceServer::new(FakeImages {
                    expected_filters,
                    reply,
                }),
            )
            .serve_with_incoming(UnixListenerStream::new(listener))
            .await
            .unwrap();
    });
    let host = format!("unix://{}", socket.display());
    let output = Command::new(env!("CARGO_BIN_EXE_risula"))
        .args(["images", "-H", &host])
        .args(args)
        .output()
        .unwrap();
    server.abort();
    output
}

fn image(name: &str, digest_char: char, created: i64, size: i64) -> Image {
    Image {
        name: name.into(),
        target: Some(Descriptor {
            digest: format!("sha256:{}", digest_char.to_string().repeat(64)),
            size,
        }),
        created_at: Some(prost_types::Timestamp {
            seconds: created,
            nanos: 0,
        }),
    }
}

fn sample_images() -> ListImagesResponse {
    ListImagesResponse {
        images: vec![
            image("busybox:latest", 'a', 1_700_000_000, 1024 * 1024),
            image("registry:5000/repo:tag", 'b', 1_800_000_000, 1500),
        ],
        ..Default::default()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn images_displays_sorted_table() {
    let output = run_images(sample_images(), HashMap::new(), &[]).await;
    assert!(output.status.success());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<&str> = stdout.lines().collect();
    assert_eq!(lines.len(), 3);
    assert!(lines[0].starts_with("REPOSITORY"));
    assert!(lines[1].starts_with("registry:5000/repo"));
    assert!(lines[1].contains("bbbbbbbbbbbb"));
    assert!(lines[1].contains("1.465KB"));
    assert!(lines[2].starts_with("busybox"));
    assert!(lines[2].contains("1.000MB"));
}

#[tokio::test(flavor = "multi_thread")]
async fn images_passes_filter_and_prints_quiet_ids() {
    let filters = HashMap::from([("reference".to_string(), "*busybox*".to_string())]);
    let output = run_images(
        sample_images(),
        filters,
        &["-f", "REFERENCE=*busybox*", "-q"],
    )
    .await;
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "bbbbbbbbbbbb\naaaaaaaaaaaa\n"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn images_reports_daemon_error() {
    let reply = ListImagesResponse {
        cc: 42,
        errmsg: "image service unavailable".into(),
        ..Default::default()
    };
    let output = run_images(reply, HashMap::new(), &[]).await;
    assert!(!output.status.success());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("42"));
    assert!(stderr.contains("image service unavailable"));
}

#[tokio::test(flavor = "multi_thread")]
async fn images_rejects_malformed_filter() {
    let output = run_images(sample_images(), HashMap::new(), &["-f", "broken"]).await;
    assert!(!output.status.success());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("expected name=value"));
}

#[tokio::test(flavor = "multi_thread")]
async fn images_handles_missing_image_metadata() {
    let reply = ListImagesResponse {
        images: vec![Image::default()],
        ..Default::default()
    };
    let table = run_images(reply.clone(), HashMap::new(), &[]).await;
    assert!(table.status.success());
    let stdout = String::from_utf8(table.stdout).unwrap();
    assert!(stdout.contains("<none>"));
    assert!(stdout.contains("0B"));

    let quiet = run_images(reply, HashMap::new(), &["-q"]).await;
    assert!(quiet.status.success());
    assert_eq!(String::from_utf8(quiet.stdout).unwrap(), "<none>      \n");
}
