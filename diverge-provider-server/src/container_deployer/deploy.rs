//! The steps of a deploy, each undone when a later one fails: the
//! caps, then the image and the mounts beside each other, then the
//! run.

use std::sync::Arc;
use std::sync::atomic::AtomicBool;

use diverge_provider_sdk::container_proxy_endpoints::OUTSIDE_PORT;
use diverge_provider_sdk::server::caller::Caller;
use diverge_provider_sdk::server::deployment::Deployment;
use futures_util::future;
use tokio::io::{AsyncReadExt as _, AsyncWriteExt as _};
use tokio::net::TcpStream;
use tokio::process::Child;
use tokio::sync::Mutex;
use tokio::time::{Duration, sleep};

use super::mounts::{Bound, bind, release};
use super::source::find;
use super::{Container, ContainerDeployer, Error, Source};
use crate::tools::{podman, start};

/// Where the proxy is bound inside every container.
const PROXY_INSIDE: &str = "/.diverge/diverge-container-proxy";

/// The whole of a deploy: the caps taken; then, beside each other,
/// the image found, pulled and counted and the volumes bound; then
/// the container run, the proxy started and answering. The caps come
/// first because they are one atomic each and a refusal there costs
/// no network; the image and the mounts have nothing to do with each
/// other, and the pull is usually the longest step, so neither waits
/// for the other. What each step made is undone by the failure of
/// any step after it.
pub(super) async fn deploy(
    deployer: &ContainerDeployer,
    deployment: &Deployment,
    name: &str,
    digest: &str,
    caller: &Caller,
) -> Result<Container, Error> {
    if !deployer.shared.disk.take(deployment.disk) {
        return Err(Error::Disk);
    }
    if !deployer.shared.memory.take(deployment.memory) {
        deployer.shared.disk.give(deployment.disk);
        return Err(Error::Memory);
    }
    match after_limits(deployer, deployment, name, digest, caller).await {
        Ok(container) => Ok(container),
        Err(error) => {
            deployer.shared.disk.give(deployment.disk);
            deployer.shared.memory.give(deployment.memory);
            Err(error)
        }
    }
}

/// The steps once the caps are taken: the image and the mounts, both
/// awaited to their end — neither is dropped mid-flight, since a bind
/// dropped mid-attach would leave a count behind — and then, both
/// made, the run; either failing undoes the other.
async fn after_limits(
    deployer: &ContainerDeployer,
    deployment: &Deployment,
    name: &str,
    digest: &str,
    caller: &Caller,
) -> Result<Container, Error> {
    let (image, bound) = future::join(
        image(deployer, name, digest, caller),
        bind(deployer, &deployment.mounts),
    )
    .await;
    let (source, image, bound) = match (image, bound) {
        (Ok((source, image)), Ok(bound)) => (source, image, bound),
        (Err(error), Ok(bound)) => {
            release(&bound).await;
            return Err(error);
        }
        (Ok((_, image)), Err(error)) => {
            deployer.shared.images.ended(&image);
            return Err(error);
        }
        (Err(error), Err(_)) => return Err(error),
    };
    match after_image(deployer, deployment, &source, &bound).await {
        Ok((name, address, proxy)) => Ok(Container {
            name,
            address,
            image,
            disk: deployment.disk,
            memory: deployment.memory,
            bound,
            proxy: Mutex::new(Some(proxy)),
            shared: Arc::clone(&deployer.shared),
            stopped: AtomicBool::new(false),
        }),
        Err(error) => {
            deployer.shared.images.ended(&image);
            release(&bound).await;
            Err(error)
        }
    }
}

/// The image: found — in the store, when the configuration lists the
/// pair; else in whichever of every listed registry and the caller
/// first answers that it holds it — pulled where it is not in the
/// store, its id read, and counted in the cache. The source and the
/// id.
async fn image(deployer: &ContainerDeployer, name: &str, digest: &str, caller: &Caller) -> Result<(Source, String), Error> {
    let source = if deployer.offered(name, digest) {
        Source::Server(format!("{name}@{digest}"))
    } else {
        find(deployer, name, digest, caller).await?
    };
    if source.pulled() {
        pull(deployer, &source).await?;
    }
    let image = podman::image_id(source.reference()).await.map_err(Error::Podman)?;
    deployer.shared.images.starting(&image).await?;
    Ok((source, image))
}

/// The steps once the image is in the store and counted: the
/// container run, its port read, the proxy started and awaited.
async fn after_image(
    deployer: &ContainerDeployer,
    deployment: &Deployment,
    source: &Source,
    bound: &[Bound],
) -> Result<(String, String, Child), Error> {
    let name = format!("diverge-{}", uuid::Uuid::new_v4());
    run(deployer, deployment, source, bound, &name).await?;
    match after_run(&name).await {
        Ok((address, proxy)) => Ok((name, address, proxy)),
        Err(error) => {
            let _ = podman::podman(["rm", "--force", "--time", "0", &name]).await;
            Err(error)
        }
    }
}

/// The steps once the container runs.
async fn after_run(name: &str) -> Result<(String, Child), Error> {
    let port = podman::port(name, OUTSIDE_PORT).await.map_err(Error::Podman)?;
    let mut proxy = start("podman", podman::command(["exec", name, PROXY_INSIDE])).map_err(Error::Run)?;
    listening(&mut proxy, port).await?;
    Ok((format!("ws://127.0.0.1:{port}"), proxy))
}

/// `podman pull` of the source: with the provider's auth file, and,
/// for the provider's own registry, without TLS, since it speaks
/// plain HTTP on the loopback. On the hosts with a machine the pull
/// from the provider's registry goes through the tunnel, which has to
/// be open still.
async fn pull(deployer: &ContainerDeployer, source: &Source) -> Result<(), Error> {
    #[cfg(not(target_os = "linux"))]
    if source.own() && !deployer.tunnel.open_still().await {
        return Err(Error::Tunnel);
    }
    let auth_file = deployer.auth_file.to_string_lossy().into_owned();
    let mut args = vec!["pull", "--authfile", &auth_file];
    if source.own() {
        args.push("--tls-verify=false");
    }
    args.push(source.reference());
    podman::podman(args)
        .await
        .map_err(Error::Pull)?
        .require("podman", |status| status.success())
        .map_err(Error::Pull)
}

/// `podman run`, detached, named, labelled as this provider's, from
/// the store alone; the request's memory as the ceiling with no swap
/// past it, its disk as the storage size; port `14979` published to a
/// loopback port podman picks; `/dev/fuse` and the mount capability
/// for the proxy's FUSE mounts; the proxy bound read-only; the
/// request's environment, entry by entry, and nothing else; every
/// volume; the image.
async fn run(
    deployer: &ContainerDeployer,
    deployment: &Deployment,
    source: &Source,
    bound: &[Bound],
    name: &str,
) -> Result<(), Error> {
    let memory = deployment.memory.to_string();
    let disk = format!("size={}", deployment.disk);
    let publish = format!("127.0.0.1::{OUTSIDE_PORT}");
    let proxy = format!("{}:{PROXY_INSIDE}:ro", deployer.proxy_path);
    let mut args: Vec<String> = [
        "run",
        "--detach",
        "--name",
        name,
        "--label",
        &deployer.label,
        "--pull",
        "never",
        "--memory",
        &memory,
        "--memory-swap",
        &memory,
        "--storage-opt",
        &disk,
        "--publish",
        &publish,
        "--device",
        "/dev/fuse",
        "--cap-add",
        "SYS_ADMIN",
        "--volume",
        &proxy,
    ]
    .iter()
    .map(|arg| arg.to_string())
    .collect();
    for (key, value) in &deployment.environment {
        args.push("--env".to_string());
        args.push(format!("{key}={value}"));
    }
    for one in bound {
        args.push("--volume".to_string());
        args.push(one.argument.clone());
    }
    args.push(source.reference().to_string());
    podman::podman(&args)
        .await
        .map_err(Error::Run)?
        .require("podman", |status| status.success())
        .map_err(Error::Run)
}

/// Wait until the proxy answers on the published port: a connection
/// accepted and a request answered with at least one byte, since on
/// the hosts with a machine the published port is accepted by the
/// machine's forwarder before anything listens behind it. Asked every
/// tenth of a second; a proxy exec that ends first is
/// [`Error::Proxy`], and nothing else ends the wait.
async fn listening(proxy: &mut Child, port: u16) -> Result<(), Error> {
    loop {
        if proxy.try_wait()?.is_some() {
            return Err(Error::Proxy);
        }
        if answers(port).await {
            return Ok(());
        }
        // A tenth of a second: the proxy binds in far less, and a
        // deploy is not made noticeably longer by it.
        sleep(Duration::from_millis(100)).await;
    }
}

/// One plain HTTP request to the port, and whether anything came
/// back.
async fn answers(port: u16) -> bool {
    let Ok(mut stream) = TcpStream::connect(("127.0.0.1", port)).await else {
        return false;
    };
    if stream
        .write_all(b"GET / HTTP/1.1\r\nHost: proxy\r\nConnection: close\r\n\r\n")
        .await
        .is_err()
    {
        return false;
    }
    let mut byte = [0u8; 1];
    matches!(stream.read(&mut byte).await, Ok(1))
}
