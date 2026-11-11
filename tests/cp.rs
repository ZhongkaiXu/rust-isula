use risula::grpc::proto::{self, ExecRequest};
use std::io::Read;
use std::process::{Command, Output};
use std::sync::{Arc, Mutex};
use tokio::net::{TcpListener, UnixListener};
use tokio_stream::wrappers::{TcpListenerStream, UnixListenerStream};
use tonic::{Request, Response, Status};
#[derive(Default, Clone)]
struct Calls {
    uploads: Vec<(serde_json::Value, Vec<u8>)>,
    downloads: Vec<proto::CopyFromContainerRequest>,
}
struct FakeCopy {
    calls: Arc<Mutex<Calls>>,
    archive: Vec<u8>,
}

fn payload() -> Vec<u8> {
    (0..131072).map(|n| n as u8).collect()
}
fn archive() -> Vec<u8> {
    let data = payload();
    let mut builder = tar::Builder::new(Vec::new());
    let mut header = tar::Header::new_gnu();
    header.set_size(data.len() as u64);
    header.set_mode(0o751);
    builder
        .append_data(&mut header, "file", data.as_slice())
        .unwrap();
    builder.into_inner().unwrap()
}
#[tonic::async_trait]
impl proto::container_service_server::ContainerService for FakeCopy {
    type LogsStream = tokio_stream::Empty<Result<proto::LogsResponse, Status>>;
    async fn logs(
        &self,
        _: Request<proto::LogsRequest>,
    ) -> Result<Response<Self::LogsStream>, Status> {
        Err(Status::unimplemented("logs"))
    }

    type CopyFromContainerStream =
        tokio_stream::Iter<std::vec::IntoIter<Result<proto::CopyFromContainerResponse, Status>>>;
    async fn copy_from_container(
        &self,
        request: Request<proto::CopyFromContainerRequest>,
    ) -> Result<Response<Self::CopyFromContainerStream>, Status> {
        let request = request.into_inner();
        self.calls.lock().unwrap().downloads.push(request.clone());
        if request.id == "rpc-error" {
            return Err(Status::not_found("missing container"));
        }
        let mut frames = vec![Ok(proto::CopyFromContainerResponse { data: vec![] })];
        for chunk in self.archive.chunks(997) {
            frames.push(Ok(proto::CopyFromContainerResponse {
                data: chunk.to_vec(),
            }));
        }
        if request.id == "stream-error" {
            frames.push(Err(Status::internal("copy failed")));
        }
        let mut response = Response::new(tokio_stream::iter(frames));
        if request.id != "no-stat" {
            response.metadata_mut().insert(
                "isulad-container-path-stat",
                if request.id == "bad-stat" {
                    "{}"
                } else {
                    "{\"Mode\":33188}"
                }
                .parse()
                .unwrap(),
            );
        }
        Ok(response)
    }
    type CopyToContainerStream =
        tokio_stream::Iter<std::vec::IntoIter<Result<proto::CopyToContainerResponse, Status>>>;
    async fn copy_to_container(
        &self,
        request: Request<tonic::Streaming<proto::CopyToContainerRequest>>,
    ) -> Result<Response<Self::CopyToContainerStream>, Status> {
        let metadata: serde_json::Value = serde_json::from_str(
            request
                .metadata()
                .get("isulad-copy-to-container")
                .unwrap()
                .to_str()
                .unwrap(),
        )
        .unwrap();
        let fail = metadata["dstPath"] == "/fail";
        let mut stream = request.into_inner();
        let mut data = Vec::new();
        while let Some(frame) = stream.message().await? {
            data.extend_from_slice(&frame.data);
        }
        self.calls.lock().unwrap().uploads.push((metadata, data));
        let mut frames = vec![Ok(proto::CopyToContainerResponse { finish: true })];
        if fail {
            frames.push(Err(Status::internal("extraction failed")));
        }
        Ok(Response::new(tokio_stream::iter(frames)))
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
        _request: Request<ExecRequest>,
    ) -> Result<Response<proto::ExecResponse>, Status> {
        Err(Status::unimplemented("exec"))
    }
    type RemoteExecStream =
        tokio_stream::Iter<std::vec::IntoIter<Result<proto::RemoteExecResponse, Status>>>;
    async fn remote_exec(
        &self,
        _request: Request<tonic::Streaming<proto::RemoteExecRequest>>,
    ) -> Result<Response<Self::RemoteExecStream>, Status> {
        Err(Status::unimplemented("remote_exec"))
    }

    async fn stats(
        &self,
        _: Request<proto::StatsRequest>,
    ) -> Result<Response<proto::StatsResponse>, Status> {
        Err(Status::unimplemented("stats"))
    }

    async fn list(
        &self,
        _: Request<proto::ListRequest>,
    ) -> Result<Response<proto::ListResponse>, Status> {
        Err(Status::unimplemented("list"))
    }

    async fn resize(
        &self,
        _: Request<proto::ResizeRequest>,
    ) -> Result<Response<proto::ResizeResponse>, Status> {
        Err(Status::unimplemented("resize"))
    }

    async fn create(
        &self,
        _request: Request<proto::CreateRequest>,
    ) -> Result<Response<proto::CreateResponse>, Status> {
        Err(Status::unimplemented("create"))
    }

    async fn restart(
        &self,
        _request: Request<proto::RestartRequest>,
    ) -> Result<Response<proto::RestartResponse>, Status> {
        Err(Status::unimplemented("restart"))
    }

    async fn start(
        &self,
        _request: Request<proto::StartRequest>,
    ) -> Result<Response<proto::StartResponse>, Status> {
        Err(Status::unimplemented("start"))
    }
    async fn inspect(
        &self,
        _request: Request<proto::InspectContainerRequest>,
    ) -> Result<Response<proto::InspectContainerResponse>, Status> {
        Err(Status::unimplemented("inspect"))
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

    async fn stop(
        &self,
        _: Request<proto::StopRequest>,
    ) -> Result<Response<proto::StopResponse>, Status> {
        Err(Status::unimplemented("stop"))
    }
    async fn wait(
        &self,
        _: Request<proto::WaitRequest>,
    ) -> Result<Response<proto::WaitResponse>, Status> {
        Err(Status::unimplemented("wait"))
    }
    async fn top(
        &self,
        _: Request<proto::TopRequest>,
    ) -> Result<Response<proto::TopResponse>, Status> {
        Err(Status::unimplemented("top"))
    }
}

async fn run_at(args: &[&str], tcp: bool) -> (Output, Calls) {
    let directory = tempfile::tempdir().unwrap();
    let socket = directory.path().join("isulad.sock");
    let listener = UnixListener::bind(&socket).unwrap();
    let tcp_listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = tcp_listener.local_addr().unwrap();
    let calls = Arc::new(Mutex::new(Calls::default()));
    let server_calls = calls.clone();
    let server = tokio::spawn(async move {
        let service = proto::container_service_server::ContainerServiceServer::new(FakeCopy {
            calls: server_calls,
            archive: archive(),
        });
        let server = tonic::transport::Server::builder().add_service(service);
        if tcp {
            server
                .serve_with_incoming(TcpListenerStream::new(tcp_listener))
                .await
                .unwrap();
        } else {
            server
                .serve_with_incoming(UnixListenerStream::new(listener))
                .await
                .unwrap();
        }
    });
    let host = if tcp {
        format!("tcp://{address}")
    } else {
        format!("unix://{}", socket.display())
    };
    let args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
    let output = tokio::task::spawn_blocking(move || {
        Command::new(env!("CARGO_BIN_EXE_risula"))
            .args(["cp", "-H", &host])
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
async fn download_reassembles_frames_and_renames_file() {
    let root = tempfile::tempdir().unwrap();
    for tcp in [false, true] {
        let dest = root.path().join("renamed");
        let (out, calls) = run_at(&["demo:/file", dest.to_str().unwrap()], tcp).await;
        assert!(out.status.success(), "{:?}", out.stderr);
        assert!(out.stdout.is_empty());
        assert_eq!(std::fs::read(&dest).unwrap(), payload());
        assert_eq!(calls.downloads[0].id, "demo");
        assert_eq!(calls.downloads[0].srcpath, "/file");
    }
}

#[tokio::test]
async fn upload_streams_archive_and_unicode_metadata() {
    let root = tempfile::tempdir().unwrap();
    let source = root.path().join("你好🌍");
    std::fs::write(&source, payload()).unwrap();
    for tcp in [false, true] {
        let (out, calls) = run_at(&[source.to_str().unwrap(), "demo:/目标"], tcp).await;
        assert!(out.status.success(), "{:?}", out.stderr);
        let (metadata, data) = &calls.uploads[0];
        assert_eq!(metadata["dstPath"], "/目标");
        assert_eq!(metadata["srcIsdir"], false);
        let mut archive = tar::Archive::new(data.as_slice());
        let mut entries = archive.entries().unwrap();
        let mut entry = entries.next().unwrap().unwrap();
        assert_eq!(entry.path().unwrap(), std::path::Path::new("你好🌍"));
        let mut bytes = Vec::new();
        entry.read_to_end(&mut bytes).unwrap();
        assert_eq!(bytes, payload());
    }
}

#[tokio::test]
async fn upload_checks_error_after_finish_frame() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("file");
    std::fs::write(&path, b"hello").unwrap();
    let (out, _) = run_at(&[path.to_str().unwrap(), "demo:/fail"], true).await;
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("extraction failed"));
}

#[tokio::test]
async fn download_failure_does_not_modify_destination() {
    let root = tempfile::tempdir().unwrap();
    let path = root.path().join("file");
    std::fs::write(&path, b"keep").unwrap();
    for name in ["rpc-error", "stream-error", "no-stat", "bad-stat"] {
        let (out, _) = run_at(&[&format!("{name}:/file"), path.to_str().unwrap()], false).await;
        assert_eq!(out.status.code(), Some(1));
        assert_eq!(std::fs::read(&path).unwrap(), b"keep");
    }
}

#[tokio::test]
async fn invalid_copy_arguments_do_not_call_transfer_rpc() {
    for args in [
        vec![],
        vec!["a"],
        vec!["a", "b"],
        vec!["a:/x", "b:/y"],
        vec!["--archive", "a", "b:/x"],
    ] {
        let (out, calls) = run_at(&args, false).await;
        assert!(!out.status.success());
        assert!(calls.uploads.is_empty() && calls.downloads.is_empty());
    }
}
