use std::process::{Command, Output};
use std::sync::{Arc, Mutex};
use std::time::Duration;

use risula::grpc::images_proto::{
    self, DeleteImageRequest, DeleteImageResponse, ImportRequest, ImportResponse,
    ListImagesRequest, ListImagesResponse, LoadImageRequest, LoadImageResponse, LoginRequest,
    LoginResponse, LogoutRequest, LogoutResponse, PullImageRequest, PullImageResponse, SearchImage,
    SearchRequest, SearchResponse, TagImageRequest, TagImageResponse,
};
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{Request, Response, Status};

struct FakeSearch {
    calls: Arc<Mutex<Vec<SearchRequest>>>,
    reply: SearchResponse,
    fail: bool,
    delay: Duration,
}

#[tonic::async_trait]
impl images_proto::images_service_server::ImagesService for FakeSearch {
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
        _request: Request<LogoutRequest>,
    ) -> Result<Response<LogoutResponse>, Status> {
        Err(Status::unimplemented("logout"))
    }

    async fn search(
        &self,
        request: Request<SearchRequest>,
    ) -> Result<Response<SearchResponse>, Status> {
        self.calls.lock().unwrap().push(request.into_inner());
        tokio::time::sleep(self.delay).await;
        if self.fail {
            return Err(Status::unavailable("registry unavailable"));
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

async fn run_search(
    args: &[&str],
    reply: SearchResponse,
    fail: bool,
    delay: Duration,
) -> (Output, Vec<SearchRequest>) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("isulad.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let server_calls = calls.clone();
    let server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(
                images_proto::images_service_server::ImagesServiceServer::new(FakeSearch {
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
        .args(["search", "-H", &host])
        .args(args)
        .output()
        .unwrap();
    server.abort();
    let seen = calls.lock().unwrap().clone();
    (output, seen)
}

fn sample_results() -> SearchResponse {
    SearchResponse {
        result_num: 2,
        search_result: vec![
            SearchImage {
                name: "low".into(),
                description: "x".repeat(60),
                star_count: 2,
                ..Default::default()
            },
            SearchImage {
                name: "high".into(),
                description: "popular image".into(),
                star_count: 10,
                is_official: true,
                ..Default::default()
            },
        ],
        ..Default::default()
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn search_sends_default_request_and_prints_sorted_table() {
    let (output, calls) = run_search(&["alpine"], sample_results(), false, Duration::ZERO).await;
    assert!(output.status.success());
    assert_eq!(calls.len(), 1);
    assert_eq!(calls[0].search_name, "alpine");
    assert_eq!(calls[0].limit, 25);
    assert!(calls[0].filters.is_empty());
    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<_> = stdout.lines().collect();
    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains("NAME"));
    assert!(lines[0].contains("DESCRIPTION"));
    assert!(lines[0].contains("OFFICIAL"));
    assert!(lines[1].starts_with("high"));
    assert!(lines[1].contains("[OK]"));
    assert!(lines[2].starts_with("low"));
    assert!(lines[2].contains(&format!("{}...", "x".repeat(44))));
    assert!(!lines[2].contains(&"x".repeat(60)));
}

#[tokio::test(flavor = "multi_thread")]
async fn search_passes_filters_limit_and_full_description() {
    let (output, calls) = run_search(
        &[
            "--limit",
            "7",
            "-f",
            "STARS=3",
            "-f",
            "is-official=true",
            "--no-trunc",
            "alpine",
        ],
        sample_results(),
        false,
        Duration::ZERO,
    )
    .await;
    assert!(output.status.success());
    assert_eq!(calls[0].limit, 7);
    assert_eq!(calls[0].filters["stars"], "3");
    assert_eq!(calls[0].filters["is-official"], "true");
    assert!(String::from_utf8(output.stdout)
        .unwrap()
        .contains(&"x".repeat(60)));
}

#[tokio::test(flavor = "multi_thread")]
async fn search_supports_custom_field_format_and_zero_limit_default() {
    let (output, calls) = run_search(
        &[
            "--limit=0",
            "--format",
            "table {{.Name}}\\t{{.StarCount}}",
            "alpine",
        ],
        sample_results(),
        false,
        Duration::ZERO,
    )
    .await;
    assert!(output.status.success());
    assert_eq!(calls[0].limit, 25);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<_> = stdout.lines().collect();
    assert_eq!(lines.len(), 3);
    assert!(lines[0].contains("NAME"));
    assert!(lines[0].contains("STARS"));
    assert!(lines[1].contains("high"));
    assert!(lines[1].contains("10"));
    assert!(lines[1].contains('\t'));
    assert!(!lines[0].contains("DESCRIPTION"));
}

#[tokio::test(flavor = "multi_thread")]
async fn search_custom_format_without_table_has_no_header() {
    let (output, calls) = run_search(
        &["--format", "{{.Name}}:{{.IsOfficial}}", "alpine"],
        sample_results(),
        false,
        Duration::ZERO,
    )
    .await;
    assert!(output.status.success());
    assert_eq!(calls.len(), 1);
    let stdout = String::from_utf8(output.stdout).unwrap();
    let lines: Vec<_> = stdout.lines().collect();
    assert_eq!(lines.len(), 2);
    assert!(lines[0].starts_with("high"));
    assert!(lines[0].contains("[OK]"));
    assert!(lines[1].starts_with("low"));
    assert!(!stdout.contains("OFFICIAL"));
}

#[tokio::test(flavor = "multi_thread")]
async fn search_rejects_invalid_limit_before_rpc() {
    let (output, calls) = run_search(
        &["--limit", "101", "alpine"],
        SearchResponse::default(),
        false,
        Duration::ZERO,
    )
    .await;
    assert!(!output.status.success());
    assert!(calls.is_empty());
    assert!(String::from_utf8(output.stderr).unwrap().contains("limit"));
}

#[tokio::test(flavor = "multi_thread")]
async fn search_rejects_invalid_filter_and_format_before_rpc() {
    for args in [
        vec!["-f", "bad", "alpine"],
        vec!["--format", "{{.Unknown}}", "alpine"],
    ] {
        let (output, calls) =
            run_search(&args, SearchResponse::default(), false, Duration::ZERO).await;
        assert!(!output.status.success());
        assert!(calls.is_empty());
    }
}

#[tokio::test(flavor = "multi_thread")]
async fn search_reports_daemon_error() {
    let reply = SearchResponse {
        cc: 42,
        errmsg: "registry search failed".into(),
        ..Default::default()
    };
    let (output, calls) = run_search(&["alpine"], reply, false, Duration::ZERO).await;
    assert!(!output.status.success());
    assert_eq!(calls.len(), 1);
    assert!(output.stdout.is_empty());
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("42"));
    assert!(stderr.contains("registry search failed"));
}

#[tokio::test(flavor = "multi_thread")]
async fn search_reports_grpc_error() {
    let (output, calls) =
        run_search(&["alpine"], SearchResponse::default(), true, Duration::ZERO).await;
    assert!(!output.status.success());
    assert_eq!(calls.len(), 1);
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("registry unavailable"));
}

#[tokio::test(flavor = "multi_thread")]
async fn search_can_run_longer_than_short_rpc_timeout() {
    let (output, calls) = run_search(
        &["alpine"],
        SearchResponse::default(),
        false,
        Duration::from_secs(6),
    )
    .await;
    assert!(output.status.success());
    assert_eq!(calls.len(), 1);
    assert!(String::from_utf8(output.stdout).unwrap().contains("NAME"));
}
