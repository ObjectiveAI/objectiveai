//! The way into the podman machine for the registry on this host.

use std::net::SocketAddr;
use std::path::Path;

use tokio::process::{Child, Command};
use tokio::sync::Mutex;
use tokio::time::{Duration, sleep};

use super::Error;
use crate::tools::{podman, start};

/// One SSH reverse tunnel into the podman machine, for the provider's
/// life: a loopback port inside the machine forwarded to the
/// registry's loopback port on this host, so podman, which pulls
/// inside the machine, reaches a registry that listens nowhere but
/// here. Opened with the machine's own SSH settings, as `podman
/// machine inspect` reports them, and the host's own `ssh`, which
/// macOS and Windows ship.
#[derive(Debug)]
pub struct Tunnel {
    /// The port inside the machine.
    port: u16,
    /// The `ssh`, held for the provider's life.
    child: Mutex<Child>,
}

impl Tunnel {
    /// Open a tunnel to `registry`, with `run_dir` holding the known
    /// hosts file the machine's key is taken on trust into. The port
    /// inside the machine is drawn at random from the ephemeral range
    /// and drawn again when the machine has it taken, which `ssh`
    /// reports by exiting, since it is told to exit on a forward it
    /// could not make; a tunnel still up after its first second is
    /// open.
    pub async fn open(run_dir: &Path, registry: SocketAddr) -> Result<Self, Error> {
        let ssh = podman::machine().await.map_err(Error::Podman)?.ok_or(Error::Tunnel)?.ssh;
        let known_hosts = run_dir.join("known_hosts");
        // Emptied at every start: the key changes when the machine is
        // remade, and there is no other host in it to keep.
        tokio::fs::write(&known_hosts, b"").await?;
        // Eight draws: the ephemeral range is 16384 ports wide, and a
        // machine with eight of them taken at random is not one.
        for _ in 0..8 {
            let port = ephemeral();
            let mut command = Command::new("ssh");
            command
                .arg("-N")
                .arg("-o")
                .arg("ExitOnForwardFailure=yes")
                .arg("-o")
                .arg("StrictHostKeyChecking=no")
                .arg("-o")
                .arg(format!("UserKnownHostsFile={}", known_hosts.display()))
                .arg("-o")
                .arg("LogLevel=ERROR")
                .arg("-i")
                .arg(&ssh.identity)
                .arg("-p")
                .arg(ssh.port.to_string())
                .arg("-R")
                .arg(format!("{port}:127.0.0.1:{}", registry.port()))
                .arg(format!("{}@127.0.0.1", ssh.user));
            let mut child = start("ssh", command).map_err(Error::Ssh)?;
            // One second: how long `ssh` takes to connect, be refused
            // a forward, and exit.
            sleep(Duration::from_secs(1)).await;
            if child.try_wait()?.is_none() {
                return Ok(Tunnel {
                    port,
                    child: Mutex::new(child),
                });
            }
        }
        Err(Error::Tunnel)
    }

    /// The port inside the machine the registry answers at.
    pub fn port(&self) -> u16 {
        self.port
    }

    /// Whether the `ssh` is still running, which is the tunnel still
    /// open.
    pub async fn open_still(&self) -> bool {
        self.child.lock().await.try_wait().ok().flatten().is_none()
    }
}

/// A port drawn from the ephemeral range, `49152` to `65535`, with
/// the randomness a UUID carries.
fn ephemeral() -> u16 {
    // The low sixteen bits of a v4 UUID's random field, offset into
    // the range: 49152 is where the range starts, 16384 how wide it is.
    49152 + (uuid::Uuid::new_v4().as_u128() as u16 % 16384)
}
