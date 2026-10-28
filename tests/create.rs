use images_proto::*;
use risula::grpc::{images_proto, proto};
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};
use tonic::{Request, Response, Status};
#[derive(Clone)]
struct FakeCreate {
    calls: Arc<Mutex<Vec<String>>>,
    requests: Arc<Mutex<Vec<proto::CreateRequest>>>,
    scenario: &'static str,
}
#[tonic::async_trait]
impl proto::container_service_server::ContainerService for FakeCreate {
    async fn resize(
        &self,
        _: Request<proto::ResizeRequest>,
    ) -> Result<Response<proto::ResizeResponse>, Status> {
        Err(Status::unimplemented("resize"))
    }

    async fn stop(
        &self,
        _request: Request<proto::StopRequest>,
    ) -> Result<Response<proto::StopResponse>, Status> {
        Err(Status::unimplemented("stop"))
    }

    async fn start(
        &self,
        _request: Request<proto::StartRequest>,
    ) -> Result<Response<proto::StartResponse>, Status> {
        self.calls.lock().unwrap().push("start".into());
        Ok(Response::new(proto::StartResponse::default()))
    }
    async fn inspect(
        &self,
        _request: Request<proto::InspectContainerRequest>,
    ) -> Result<Response<proto::InspectContainerResponse>, Status> {
        Err(Status::unimplemented("inspect"))
    }
    async fn wait(
        &self,
        _request: Request<proto::WaitRequest>,
    ) -> Result<Response<proto::WaitResponse>, Status> {
        Err(Status::unimplemented("wait"))
    }
    type RemoteStartStream = tokio_stream::Empty<Result<proto::RemoteStartResponse, Status>>;
    async fn remote_start(
        &self,
        _request: Request<tonic::Streaming<proto::RemoteStartRequest>>,
    ) -> Result<Response<Self::RemoteStartStream>, Status> {
        Err(Status::unimplemented("remote_start"))
    }

    async fn rename(
        &self,
        _request: Request<proto::RenameRequest>,
    ) -> Result<Response<proto::RenameResponse>, Status> {
        Err(Status::unimplemented("rename"))
    }

    async fn resume(
        &self,
        _request: Request<proto::ResumeRequest>,
    ) -> Result<Response<proto::ResumeResponse>, Status> {
        Err(Status::unimplemented("resume"))
    }

    async fn pause(
        &self,
        _request: Request<proto::PauseRequest>,
    ) -> Result<Response<proto::PauseResponse>, Status> {
        Err(Status::unimplemented("pause"))
    }

    async fn kill(
        &self,
        _request: Request<proto::KillRequest>,
    ) -> Result<Response<proto::KillResponse>, Status> {
        Err(Status::unimplemented("kill"))
    }

    async fn delete(
        &self,
        _request: Request<proto::DeleteRequest>,
    ) -> Result<Response<proto::DeleteResponse>, Status> {
        Err(Status::unimplemented("delete"))
    }

    async fn version(
        &self,
        _request: Request<proto::VersionRequest>,
    ) -> Result<Response<proto::VersionResponse>, Status> {
        Err(Status::unimplemented("version"))
    }

    async fn info(
        &self,
        _request: Request<proto::InfoRequest>,
    ) -> Result<Response<proto::InfoResponse>, Status> {
        Err(Status::unimplemented("info"))
    }

    async fn restart(
        &self,
        _: Request<proto::RestartRequest>,
    ) -> Result<Response<proto::RestartResponse>, Status> {
        Err(Status::unimplemented("restart"))
    }
    async fn create(
        &self,
        request: Request<proto::CreateRequest>,
    ) -> Result<Response<proto::CreateResponse>, Status> {
        let r = request.into_inner();
        self.calls.lock().unwrap().push("create".into());
        let first = {
            let mut requests = self.requests.lock().unwrap();
            requests.push(r);
            requests.len() == 1
        };
        if self.scenario == "transport" {
            return Err(Status::unavailable("unavailable"));
        }
        if self.scenario == "slow" {
            tokio::time::sleep(std::time::Duration::from_secs(6)).await;
        }
        let missing = self.scenario == "still-missing"
            || (first && matches!(self.scenario, "missing" | "pull-fails" | "pull-empty"));
        Ok(Response::new(proto::CreateResponse {
            cc: if missing || self.scenario == "conflict" {
                42
            } else {
                0
            },
            errmsg: if missing {
                "No such image: example:test".into()
            } else if self.scenario == "conflict" {
                "name already in use".into()
            } else {
                "success warning".into()
            },
            id: if self.scenario == "no-id" {
                String::new()
            } else {
                "container-id".into()
            },
            ..Default::default()
        }))
    }
}
#[tonic::async_trait]
impl images_proto::images_service_server::ImagesService for FakeCreate {
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
        _request: Request<SearchRequest>,
    ) -> Result<Response<SearchResponse>, Status> {
        Err(Status::unimplemented("search"))
    }

    type PullImageStream =
        tokio_stream::Iter<std::vec::IntoIter<Result<PullImageResponse, Status>>>;
    async fn pull_image(
        &self,
        r: Request<PullImageRequest>,
    ) -> Result<Response<Self::PullImageStream>, Status> {
        let r = r.into_inner();
        assert_eq!(r.image.unwrap().image, "example:test");
        assert!(r.auth.is_none());
        assert!(!r.is_progress_visible);
        self.calls.lock().unwrap().push("pull".into());
        let replies = if self.scenario == "pull-fails" {
            vec![Err(Status::unavailable("registry unavailable"))]
        } else if self.scenario == "pull-empty" {
            vec![]
        } else {
            vec![Ok(PullImageResponse {
                image_ref: "example:test".into(),
                ..Default::default()
            })]
        };
        Ok(Response::new(tokio_stream::iter(replies)))
    }
}
async fn run(
    args: &[&str],
    scenario: &'static str,
) -> (Output, Vec<String>, Vec<proto::CreateRequest>) {
    run_command("create", args, scenario).await
}

async fn run_command(
    command: &'static str,
    args: &[&str],
    scenario: &'static str,
) -> (Output, Vec<String>, Vec<proto::CreateRequest>) {
    let dir = tempfile::tempdir().unwrap();
    let socket = dir.path().join("daemon.sock");
    let listener = tokio::net::UnixListener::bind(&socket).unwrap();
    let fake = FakeCreate {
        calls: Arc::default(),
        requests: Arc::default(),
        scenario,
    };
    let service = fake.clone();
    let server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(
                proto::container_service_server::ContainerServiceServer::new(service.clone()),
            )
            .add_service(images_proto::images_service_server::ImagesServiceServer::new(service))
            .serve_with_incoming(tokio_stream::wrappers::UnixListenerStream::new(listener))
            .await
            .unwrap();
    });
    let host = format!("unix://{}", socket.display());
    let args: Vec<_> = args.iter().map(|s| s.to_string()).collect();
    let output = tokio::task::spawn_blocking(move || {
        Command::new(env!("CARGO_BIN_EXE_risula"))
            .args([command, "-H", &host])
            .args(args)
            .output()
            .unwrap()
    })
    .await
    .unwrap();
    server.abort();
    let calls = fake.calls.lock().unwrap().clone();
    let requests = fake.requests.lock().unwrap().clone();
    (output, calls, requests)
}
#[tokio::test]
async fn pull_policies_and_retry_order() {
    for (policy, scenario, expected, code) in [
        ("missing", "ok", vec!["create"], 0),
        ("always", "ok", vec!["pull", "create"], 0),
        ("never", "missing", vec!["create"], 1),
        ("missing", "missing", vec!["create", "pull", "create"], 0),
        (
            "missing",
            "still-missing",
            vec!["create", "pull", "create"],
            1,
        ),
        ("always", "pull-fails", vec!["pull"], 1),
        ("missing", "pull-fails", vec!["create", "pull"], 1),
        ("missing", "pull-empty", vec!["create", "pull"], 1),
        ("missing", "conflict", vec!["create"], 1),
        ("missing", "transport", vec!["create"], 1),
        ("never", "no-id", vec!["create"], 1),
    ] {
        let (out, calls, requests) = run(
            &[
                &format!("--pull={policy}"),
                "--name=demo",
                "example:test",
                "echo",
                "hello",
            ],
            scenario,
        )
        .await;
        assert_eq!(
            out.status.code(),
            Some(code),
            "{policy} {scenario}: {out:?}"
        );
        assert_eq!(calls, expected);
        if code == 0 {
            assert_eq!(out.stdout, b"container-id\n");
        } else {
            assert!(out.stdout.is_empty());
        }
        if requests.len() == 2 {
            assert_eq!(requests[0], requests[1]);
        }
    }
}
#[tokio::test]
async fn options_after_image_are_command_arguments() {
    let (out, _, r) = run(
        &[
            "-it",
            "-m=64m",
            "--network=none",
            "example:test",
            "sh",
            "-c",
            "echo --name=test",
        ],
        "ok",
    )
    .await;
    assert!(out.status.success(), "{out:?}");
    let c: serde_json::Value = serde_json::from_str(&r[0].customconfig).unwrap();
    assert_eq!(
        c["Cmd"],
        serde_json::json!(["sh", "-c", "echo --name=test"])
    );
    assert_eq!(c["Tty"], true);
    assert_eq!(c["OpenStdin"], true);
}
#[tokio::test]
async fn invalid_arguments_never_call_daemon() {
    for args in [
        vec![],
        vec!["--unknown", "example:test"],
        vec!["--name=bad/name", "example:test"],
        vec!["--pull=bad", "example:test"],
        vec!["-m64m", "example:test"],
        vec!["--tty=maybe", "example:test"],
        vec!["--rm", "--restart=always", "example:test"],
    ] {
        let (out, calls, _) = run(&args, "ok").await;
        assert_eq!(out.status.code(), Some(125), "{args:?} {out:?}");
        assert!(calls.is_empty());
    }
}
#[tokio::test]
async fn create_waits_for_slow_daemon() {
    let (out, calls, _) = run(&["example:test"], "slow").await;
    assert!(out.status.success(), "{out:?}");
    assert_eq!(calls, ["create"]);
}
#[tokio::test]
async fn help_lists_create_options() {
    let (out, calls, _) = run(&["--help"], "ok").await;
    assert!(out.status.success());
    assert!(calls.is_empty());
    let help = String::from_utf8(out.stdout).unwrap();
    for option in [
        "--pull",
        "--network",
        "--mount",
        "--health-cmd",
        "--memory",
        "--external-rootfs",
    ] {
        assert!(help.contains(option));
    }
}

#[tokio::test]
async fn run_reuses_pull_policies_and_only_starts_after_creation() {
    for (policy, scenario, expected, code) in [
        ("missing", "ok", vec!["create", "start"], 0),
        ("always", "ok", vec!["pull", "create", "start"], 0),
        ("never", "missing", vec!["create"], 1),
        (
            "missing",
            "missing",
            vec!["create", "pull", "create", "start"],
            0,
        ),
        (
            "missing",
            "still-missing",
            vec!["create", "pull", "create"],
            1,
        ),
        ("always", "pull-fails", vec!["pull"], 1),
        ("missing", "pull-fails", vec!["create", "pull"], 1),
    ] {
        let (out, calls, _) = run_command(
            "run",
            &["-d", &format!("--pull={policy}"), "example:test"],
            scenario,
        )
        .await;
        assert_eq!(
            out.status.code(),
            Some(code),
            "{policy} {scenario}: {out:?}"
        );
        assert_eq!(calls, expected);
        if code == 0 {
            assert!(out.stdout.ends_with(b"container-id\n"));
        }
    }
}
