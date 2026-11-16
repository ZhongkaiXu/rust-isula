use risula::grpc::{images_proto, proto};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};
use tokio::net::UnixListener;
use tokio_stream::wrappers::UnixListenerStream;
use tonic::{Request, Response, Status};
type Call = (String, String, bool, i32);
#[derive(Clone)]
struct FakeInspect {
    calls: Arc<Mutex<Vec<Call>>>,
}
const CONTAINER: &str = r#"{"Id":"container-id","Name":"demo","State":{"Running":false,"ExitCode":7},"Config":{"Env":["A=1","B=2"]},"Mounts":[{"Source":"/first"},{"Source":"/second"}],"Empty":null}"#;
const IMAGE: &str = r#"{"image":{"id":"image-id","repo_tags":["busybox:latest"]}}"#;

#[tonic::async_trait]
impl proto::container_service_server::ContainerService for FakeInspect {
    async fn update(
        &self,
        _: Request<proto::UpdateRequest>,
    ) -> Result<Response<proto::UpdateResponse>, Status> {
        Err(Status::unimplemented("update"))
    }
    async fn export(
        &self,
        _: Request<proto::ExportRequest>,
    ) -> Result<Response<proto::ExportResponse>, Status> {
        Err(Status::unimplemented("export"))
    }
    type EventsStream = tokio_stream::Empty<Result<proto::Event, Status>>;
    async fn events(
        &self,
        _: Request<proto::EventsRequest>,
    ) -> Result<Response<Self::EventsStream>, Status> {
        Err(Status::unimplemented("events"))
    }
    type LogsStream = tokio_stream::Empty<Result<proto::LogsResponse, Status>>;
    async fn logs(
        &self,
        _: Request<proto::LogsRequest>,
    ) -> Result<Response<Self::LogsStream>, Status> {
        Err(Status::unimplemented("logs"))
    }

    type CopyFromContainerStream =
        tokio_stream::Empty<Result<proto::CopyFromContainerResponse, Status>>;
    async fn copy_from_container(
        &self,
        _: Request<proto::CopyFromContainerRequest>,
    ) -> Result<Response<Self::CopyFromContainerStream>, Status> {
        Err(Status::unimplemented("copy_from_container"))
    }
    type CopyToContainerStream =
        tokio_stream::Empty<Result<proto::CopyToContainerResponse, Status>>;
    async fn copy_to_container(
        &self,
        _: Request<tonic::Streaming<proto::CopyToContainerRequest>>,
    ) -> Result<Response<Self::CopyToContainerStream>, Status> {
        Err(Status::unimplemented("copy_to_container"))
    }

    type AttachStream = tokio_stream::Empty<Result<proto::AttachResponse, Status>>;
    async fn attach(
        &self,
        _: Request<tonic::Streaming<proto::AttachRequest>>,
    ) -> Result<Response<Self::AttachStream>, Status> {
        Err(Status::unimplemented("attach"))
    }

    async fn exec(
        &self,
        _: Request<proto::ExecRequest>,
    ) -> Result<Response<proto::ExecResponse>, Status> {
        Err(Status::unimplemented("exec"))
    }
    type RemoteExecStream = tokio_stream::Empty<Result<proto::RemoteExecResponse, Status>>;
    async fn remote_exec(
        &self,
        _: Request<tonic::Streaming<proto::RemoteExecRequest>>,
    ) -> Result<Response<Self::RemoteExecStream>, Status> {
        Err(Status::unimplemented("remote_exec"))
    }

    async fn stats(
        &self,
        _: Request<proto::StatsRequest>,
    ) -> Result<Response<proto::StatsResponse>, Status> {
        Err(Status::unimplemented("stats"))
    }

    async fn top(
        &self,
        _: Request<proto::TopRequest>,
    ) -> Result<Response<proto::TopResponse>, Status> {
        Err(Status::unimplemented("top"))
    }

    async fn list(
        &self,
        _: Request<proto::ListRequest>,
    ) -> Result<Response<proto::ListResponse>, Status> {
        Err(Status::unimplemented("list"))
    }
    async fn create(
        &self,
        _: Request<proto::CreateRequest>,
    ) -> Result<Response<proto::CreateResponse>, Status> {
        Err(Status::unimplemented("create"))
    }
    async fn version(
        &self,
        _: Request<proto::VersionRequest>,
    ) -> Result<Response<proto::VersionResponse>, Status> {
        Err(Status::unimplemented("version"))
    }
    async fn info(
        &self,
        _: Request<proto::InfoRequest>,
    ) -> Result<Response<proto::InfoResponse>, Status> {
        Err(Status::unimplemented("info"))
    }
    async fn stop(
        &self,
        _: Request<proto::StopRequest>,
    ) -> Result<Response<proto::StopResponse>, Status> {
        Err(Status::unimplemented("stop"))
    }
    async fn restart(
        &self,
        _: Request<proto::RestartRequest>,
    ) -> Result<Response<proto::RestartResponse>, Status> {
        Err(Status::unimplemented("restart"))
    }
    async fn delete(
        &self,
        _: Request<proto::DeleteRequest>,
    ) -> Result<Response<proto::DeleteResponse>, Status> {
        Err(Status::unimplemented("delete"))
    }
    async fn kill(
        &self,
        _: Request<proto::KillRequest>,
    ) -> Result<Response<proto::KillResponse>, Status> {
        Err(Status::unimplemented("kill"))
    }
    async fn pause(
        &self,
        _: Request<proto::PauseRequest>,
    ) -> Result<Response<proto::PauseResponse>, Status> {
        Err(Status::unimplemented("pause"))
    }
    async fn resume(
        &self,
        _: Request<proto::ResumeRequest>,
    ) -> Result<Response<proto::ResumeResponse>, Status> {
        Err(Status::unimplemented("resume"))
    }
    async fn rename(
        &self,
        _: Request<proto::RenameRequest>,
    ) -> Result<Response<proto::RenameResponse>, Status> {
        Err(Status::unimplemented("rename"))
    }
    async fn start(
        &self,
        _: Request<proto::StartRequest>,
    ) -> Result<Response<proto::StartResponse>, Status> {
        Err(Status::unimplemented("start"))
    }
    type RemoteStartStream = tokio_stream::Empty<Result<proto::RemoteStartResponse, Status>>;
    async fn remote_start(
        &self,
        _: Request<tonic::Streaming<proto::RemoteStartRequest>>,
    ) -> Result<Response<Self::RemoteStartStream>, Status> {
        Err(Status::unimplemented("remote_start"))
    }
    async fn inspect(
        &self,
        request: Request<proto::InspectContainerRequest>,
    ) -> Result<Response<proto::InspectContainerResponse>, Status> {
        let req = request.into_inner();
        self.calls.lock().unwrap().push((
            "container".into(),
            req.id.clone(),
            req.bformat,
            req.timeout,
        ));

        if req.id == "transport" {
            return Err(Status::unavailable("connection lost"));
        }
        if req.id == "slow" {
            tokio::time::sleep(std::time::Duration::from_secs(6)).await;
        }
        let (cc, errmsg) = match req.id.as_str() {
            "image" | "missing" => (1, "No such image or container or accelerator"),
            "invalid:name" => (1, "Inspect invalid name"),
            "denied" => (2, "permission denied"),
            _ => (0, ""),
        };
        Ok(Response::new(proto::InspectContainerResponse {
            container_json: if req.id == "badjson" {
                "broken"
            } else {
                CONTAINER
            }
            .into(),
            cc,
            errmsg: errmsg.into(),
        }))
    }
    async fn resize(
        &self,
        _: Request<proto::ResizeRequest>,
    ) -> Result<Response<proto::ResizeResponse>, Status> {
        Err(Status::unimplemented("resize"))
    }
    async fn wait(
        &self,
        _: Request<proto::WaitRequest>,
    ) -> Result<Response<proto::WaitResponse>, Status> {
        Err(Status::unimplemented("wait"))
    }
}

#[tonic::async_trait]
impl images_proto::images_service_server::ImagesService for FakeInspect {
    async fn inspect(
        &self,
        request: Request<images_proto::InspectImageRequest>,
    ) -> Result<Response<images_proto::InspectImageResponse>, Status> {
        let req = request.into_inner();
        self.calls.lock().unwrap().push((
            "images".into(),
            req.id.clone(),
            req.bformat,
            req.timeout,
        ));
        Ok(Response::new(images_proto::InspectImageResponse {
            image_json: IMAGE.into(),
            cc: u32::from(req.id == "missing"),
            errmsg: if req.id == "missing" {
                "No such image"
            } else {
                ""
            }
            .into(),
        }))
    }
    async fn list(
        &self,
        _: Request<images_proto::ListImagesRequest>,
    ) -> Result<Response<images_proto::ListImagesResponse>, Status> {
        Err(Status::unimplemented("list"))
    }
    async fn delete(
        &self,
        _: Request<images_proto::DeleteImageRequest>,
    ) -> Result<Response<images_proto::DeleteImageResponse>, Status> {
        Err(Status::unimplemented("delete"))
    }
    async fn tag(
        &self,
        _: Request<images_proto::TagImageRequest>,
    ) -> Result<Response<images_proto::TagImageResponse>, Status> {
        Err(Status::unimplemented("tag"))
    }
    async fn load(
        &self,
        _: Request<images_proto::LoadImageRequest>,
    ) -> Result<Response<images_proto::LoadImageResponse>, Status> {
        Err(Status::unimplemented("load"))
    }
    async fn import(
        &self,
        _: Request<images_proto::ImportRequest>,
    ) -> Result<Response<images_proto::ImportResponse>, Status> {
        Err(Status::unimplemented("import"))
    }
    async fn login(
        &self,
        _: Request<images_proto::LoginRequest>,
    ) -> Result<Response<images_proto::LoginResponse>, Status> {
        Err(Status::unimplemented("login"))
    }
    async fn logout(
        &self,
        _: Request<images_proto::LogoutRequest>,
    ) -> Result<Response<images_proto::LogoutResponse>, Status> {
        Err(Status::unimplemented("logout"))
    }
    async fn search(
        &self,
        _: Request<images_proto::SearchRequest>,
    ) -> Result<Response<images_proto::SearchResponse>, Status> {
        Err(Status::unimplemented("search"))
    }
    type PullImageStream = tokio_stream::Empty<Result<images_proto::PullImageResponse, Status>>;
    async fn pull_image(
        &self,
        _: Request<images_proto::PullImageRequest>,
    ) -> Result<Response<Self::PullImageStream>, Status> {
        Err(Status::unimplemented("pull_image"))
    }
}

async fn inspect(args: &[&str]) -> (Output, Vec<Call>) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("isulad.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let calls = Arc::new(Mutex::new(Vec::new()));
    let service = FakeInspect {
        calls: calls.clone(),
    };
    let server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(
                proto::container_service_server::ContainerServiceServer::new(service.clone()),
            )
            .add_service(images_proto::images_service_server::ImagesServiceServer::new(service))
            .serve_with_incoming(UnixListenerStream::new(listener))
            .await
            .unwrap();
    });
    let host = format!("unix://{}", socket.display());
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let output = tokio::task::spawn_blocking(move || {
        Command::new(env!("CARGO_BIN_EXE_risula"))
            .args(["inspect", "-H", &host])
            .args(args)
            .output()
            .unwrap()
    })
    .await
    .unwrap();
    server.abort();
    let calls = calls.lock().unwrap().clone();
    (output, calls)
}

#[tokio::test]
async fn inspect_container_and_image_in_one_array() {
    let (output, calls) = inspect(&["demo", "image"]).await;
    assert!(output.status.success());
    let value: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(value[0]["Id"], "container-id");
    assert_eq!(value[1]["image"]["id"], "image-id");
    assert_eq!(
        calls,
        vec![
            ("container".into(), "demo".into(), false, 120),
            ("container".into(), "image".into(), false, 120),
            ("images".into(), "image".into(), false, 120)
        ]
    );
    assert!(String::from_utf8(output.stdout)
        .unwrap()
        .starts_with("[\n    {\n        \"Id\": "));
}

#[tokio::test]
async fn inspect_format_quotes_each_field_and_propagates_timeout() {
    let (output, calls) = inspect(&[
        "-t",
        "0x10",
        "-f",
        "{{json .Name}} {{.State.Running}} {{.Config.Env}} {{.Empty}}",
        "demo",
    ])
    .await;
    assert!(output.status.success());
    assert_eq!(
        String::from_utf8(output.stdout).unwrap(),
        "\"demo\"\nfalse\n[\n    A=1,\n    B=2\n]\nnull\n"
    );
    assert_eq!(calls[0], ("container".into(), "demo".into(), true, 16));
}

#[tokio::test]
async fn inspect_array_path_and_multiple_objects() {
    let (output, _) = inspect(&["-f", "{{.Mounts.Source}}", "first", "second"]).await;
    assert!(output.status.success());
    assert_eq!(output.stdout, b"/first\n/first\n");
}

#[tokio::test]
async fn inspect_invalid_container_name_tries_image() {
    let (output, calls) = inspect(&["-t", "-1", "-f", "{{.image.id}}", "invalid:name"]).await;
    assert!(output.status.success());
    assert_eq!(output.stdout, b"image-id\n");
    assert_eq!(calls[1], ("images".into(), "invalid:name".into(), true, -1));
}

#[tokio::test]
async fn inspect_errors_have_no_partial_output_or_unrelated_fallback() {
    for name in ["denied", "transport", "badjson", "missing"] {
        let (output, calls) = inspect(&["demo", name, "unqueried"]).await;
        assert_eq!(output.status.code(), Some(1), "{name}");
        assert!(output.stdout.is_empty());
        assert!(!output.stderr.is_empty());
        assert_eq!(calls.len(), if name == "missing" { 3 } else { 2 });
    }
}

#[tokio::test]
async fn inspect_missing_field_returns_error_without_output() {
    let (output, _) = inspect(&["-f", "{{.Name}} {{.NoSuchField}}", "demo"]).await;
    assert_eq!(output.status.code(), Some(1));
    assert!(output.stdout.is_empty());
    assert!(String::from_utf8(output.stderr)
        .unwrap()
        .contains("NoSuchField"));
}

#[tokio::test]
async fn inspect_argument_errors_do_not_call_daemon() {
    for (args, code) in [
        (vec![], 125),
        (vec!["-t", "bad", "demo"], 125),
        (vec!["-f", "{{.Name}}text", "demo"], 1),
        (vec!["-f", "", "demo"], 1),
        (vec!["-f", "{{range .Config}}", "demo"], 1),
        (vec!["--bad", "demo"], 125),
    ] {
        let (output, calls) = inspect(&args).await;
        assert_eq!(output.status.code(), Some(code), "{args:?}");
        assert!(calls.is_empty());
    }
    let names = vec!["demo"; 1000];
    let (output, calls) = inspect(&names).await;
    assert_eq!(output.status.code(), Some(125));
    assert!(calls.is_empty());
}

#[tokio::test]
async fn inspect_waits_beyond_default_transport_timeout() {
    let (output, _) = inspect(&["-f", "{{.Id}}", "slow"]).await;
    assert!(output.status.success());
    assert_eq!(output.stdout, b"container-id\n");
}
