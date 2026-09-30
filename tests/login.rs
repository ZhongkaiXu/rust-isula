use std::io::Write;
use std::process::{Command, Output, Stdio};
use std::sync::{Arc, Mutex};

use risula::grpc::images_proto::{
    self, DeleteImageRequest, DeleteImageResponse, ImportRequest, ImportResponse,
    ListImagesRequest, ListImagesResponse, LoadImageRequest, LoadImageResponse, LoginRequest,
    LoginResponse, LogoutRequest, LogoutResponse, PullImageRequest, PullImageResponse,
    TagImageRequest, TagImageResponse,
};
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{Request, Response, Status};

struct FakeLogin {
    calls: Arc<Mutex<Vec<LoginRequest>>>,
    reply: LoginResponse,
    fail: bool,
}

#[tonic::async_trait]
impl images_proto::images_service_server::ImagesService for FakeLogin {
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
        request: Request<LoginRequest>,
    ) -> Result<Response<LoginResponse>, Status> {
        self.calls.lock().unwrap().push(request.into_inner());
        if self.fail {
            return Err(Status::unavailable("registry service unavailable"));
        }
        Ok(Response::new(self.reply.clone()))
    }

    async fn logout(
        &self,
        _request: Request<LogoutRequest>,
    ) -> Result<Response<LogoutResponse>, Status> {
        Err(Status::unimplemented("logout"))
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

async fn run_login(
    args: &[&str],
    input: &str,
    reply: LoginResponse,
    fail: bool,
) -> (Output, Vec<LoginRequest>) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("isulad.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let server_calls = calls.clone();
    let server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(
                images_proto::images_service_server::ImagesServiceServer::new(FakeLogin {
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
    let mut child = Command::new(env!("CARGO_BIN_EXE_risula"))
        .args(["login", "-H", &host])
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    child
        .stdin
        .take()
        .unwrap()
        .write_all(input.as_bytes())
        .unwrap();
    let output = child.wait_with_output().unwrap();
    server.abort();
    let seen = calls.lock().unwrap().clone();
    (output, seen)
}

#[tokio::test(flavor = "multi_thread")]
async fn login_reads_password_stdin_and_sends_oci_request() {
    let (output, calls) = run_login(
        &["-u", "alice", "--password-stdin", "registry.test"],
        "test-secret\r\n",
        LoginResponse::default(),
        false,
    )
    .await;
    assert!(output.status.success());
    assert_eq!(
        calls,
        vec![LoginRequest {
            username: "alice".into(),
            password: "test-secret".into(),
            server: "registry.test".into(),
            r#type: "oci".into(),
        }]
    );
    assert!(output.stdout.is_empty());
    assert_eq!(
        String::from_utf8(output.stderr).unwrap(),
        "Login Succeeded\n"
    );
}

#[tokio::test(flavor = "multi_thread")]
async fn login_password_option_warns_without_printing_password() {
    let (output, calls) = run_login(
        &["-u", "alice", "-p", "test-secret", "registry.test"],
        "",
        LoginResponse::default(),
        false,
    )
    .await;
    assert!(output.status.success());
    assert_eq!(calls.len(), 1);
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("--password-stdin"));
    assert!(!stdout.contains("test-secret"));
}

#[tokio::test(flavor = "multi_thread")]
async fn login_rejects_conflicting_password_options_before_rpc() {
    let (output, calls) = run_login(
        &[
            "-u",
            "alice",
            "-p",
            "test-secret",
            "--password-stdin",
            "registry.test",
        ],
        "",
        LoginResponse::default(),
        false,
    )
    .await;
    assert!(!output.status.success());
    assert!(calls.is_empty());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("mutually exclusive"));
}

#[tokio::test(flavor = "multi_thread")]
async fn login_requires_username_with_password_stdin() {
    let (output, calls) = run_login(
        &["--password-stdin", "registry.test"],
        "test-secret\n",
        LoginResponse::default(),
        false,
    )
    .await;
    assert!(!output.status.success());
    assert!(calls.is_empty());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("requires --username"));
}

#[tokio::test(flavor = "multi_thread")]
async fn login_without_password_cannot_prompt_on_pipe() {
    let (output, calls) = run_login(
        &["-u", "alice", "registry.test"],
        "",
        LoginResponse::default(),
        false,
    )
    .await;
    assert!(!output.status.success());
    assert!(calls.is_empty());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("non TTY"));
}

#[tokio::test(flavor = "multi_thread")]
async fn login_rejects_empty_password_stdin() {
    let (output, calls) = run_login(
        &["-u", "alice", "--password-stdin", "registry.test"],
        "\n",
        LoginResponse::default(),
        false,
    )
    .await;
    assert!(!output.status.success());
    assert!(calls.is_empty());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("password must be"));
}

#[tokio::test(flavor = "multi_thread")]
async fn login_rejects_plain_tcp_before_sending_credentials() {
    let (output, calls) = run_login(
        &[
            "-H",
            "tcp://127.0.0.1:65534",
            "-u",
            "alice",
            "--password-stdin",
            "registry.test",
        ],
        "test-secret\n",
        LoginResponse::default(),
        false,
    )
    .await;
    assert!(!output.status.success());
    assert!(calls.is_empty());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("TCP credentials are not encrypted"));
}

#[tokio::test(flavor = "multi_thread")]
async fn login_reports_daemon_error() {
    let (output, calls) = run_login(
        &["-u", "alice", "--password-stdin", "registry.test"],
        "test-secret\n",
        LoginResponse {
            cc: 42,
            errmsg: "authentication failed".into(),
        },
        false,
    )
    .await;
    assert!(!output.status.success());
    assert_eq!(calls.len(), 1);
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(stderr.contains("42"));
    assert!(stderr.contains("authentication failed"));
    assert!(!stderr.contains("test-secret"));
}

#[tokio::test(flavor = "multi_thread")]
async fn login_reports_grpc_error() {
    let (output, calls) = run_login(
        &["-u", "alice", "--password-stdin", "registry.test"],
        "test-secret\n",
        LoginResponse::default(),
        true,
    )
    .await;
    assert!(!output.status.success());
    assert_eq!(calls.len(), 1);
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("registry service unavailable"));
}
