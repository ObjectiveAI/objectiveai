use bollard::exec::CreateExecOptions;
use bollard::models::ContainerCreateBody;
use bollard::query_parameters::{CreateContainerOptionsBuilder, UploadToContainerOptionsBuilder};

use crate::ctx;

const MCP_CONTAINER_PORT: &str = "3000/tcp";

pub struct Orchestrator {
    pub docker_timeout: u64,
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("docker error: {0}")]
    Docker(#[from] bollard::errors::Error),
    #[error("port timeout: {0}")]
    PortTimeout(String),
}

impl objectiveai_sdk::error::StatusError for Error {
    fn status(&self) -> u16 {
        500
    }

    fn message(&self) -> Option<serde_json::Value> {
        Some(serde_json::json!({
            "kind": "orchestrator",
            "error": {
                "kind": "bollard",
                "error": self.to_string(),
            }
        }))
    }
}

impl<CTXEXT: Send + Sync + 'static> super::Orchestrator<CTXEXT> for Orchestrator {
    type Error = Error;

    fn spawn_containers(
        &self,
        _ctx: &ctx::Context<CTXEXT, impl ctx::persistent_cache::PersistentCacheClient>,
        image: &str,
        num_builders: usize,
        execution_id: &str,
        binaries: &[(&str, &[u8])],
        env: &[(&str, &str)],
    ) -> impl std::future::Future<Output = Result<Vec<String>, Self::Error>> + Send {
        let image = image.to_string();
        let execution_id = execution_id.to_string();
        let docker_timeout = self.docker_timeout;
        let tar_bytes = build_tar(binaries);
        let binary_names: Vec<String> = binaries.iter().map(|(name, _)| format!("/{name}")).collect();
        let env_vec: Vec<String> = env.iter().map(|(k, v)| format!("{k}={v}")).collect();

        async move {
            let docker_host = std::env::var("DOCKER_HOST").unwrap_or_else(|_| {
                #[cfg(unix)] { "unix:///var/run/docker.sock".to_string() }
                #[cfg(windows)] { "npipe:////./pipe/docker_engine".to_string() }
            });
            let docker = bollard::Docker::connect_with_local(
                &docker_host, docker_timeout, bollard::API_DEFAULT_VERSION,
            )?;

            let futs: Vec<_> = (0..num_builders)
                .map(|i| spawn_builder(&docker, &image, i, &execution_id, &tar_bytes, &binary_names, &env_vec))
                .collect();

            let host_ports = futures::future::try_join_all(futs).await?;

            Ok(host_ports
                .into_iter()
                .map(|port| format!("http://localhost:{port}"))
                .collect())
        }
    }

    fn cleanup_containers(
        &self,
        _ctx: &ctx::Context<CTXEXT, impl ctx::persistent_cache::PersistentCacheClient>,
        execution_id: &str,
        num_builders: usize,
    ) -> impl std::future::Future<Output = ()> + Send {
        let execution_id = execution_id.to_string();
        let docker_timeout = self.docker_timeout;

        async move {
            let docker_host = std::env::var("DOCKER_HOST").unwrap_or_else(|_| {
                #[cfg(unix)] { "unix:///var/run/docker.sock".to_string() }
                #[cfg(windows)] { "npipe:////./pipe/docker_engine".to_string() }
            });
            let Ok(docker) = bollard::Docker::connect_with_local(
                &docker_host, docker_timeout, bollard::API_DEFAULT_VERSION,
            ) else {
                return;
            };

            for i in 0..num_builders {
                let name = format!("objectiveai-{execution_id}-{i}");
                let _ = docker.stop_container(&name, None).await;
                let _ = docker.remove_container(&name, None).await;
            }
        }
    }
}

/// Build a tar archive containing all binaries at the archive root.
fn build_tar(binaries: &[(&str, &[u8])]) -> Vec<u8> {
    let mut ar = tar::Builder::new(Vec::new());
    for (name, data) in binaries {
        let mut header = tar::Header::new_gnu();
        header.set_size(data.len() as u64);
        header.set_mode(0o755);
        header.set_cksum();
        ar.append_data(&mut header, name, *data)
            .expect("failed to build tar archive");
    }
    ar.into_inner().expect("failed to finalize tar archive")
}

/// Spawn a single builder container: create, start, upload binaries, and
/// execute each binary. Returns the host port that the MCP server is exposed on.
async fn spawn_builder(
    docker: &bollard::Docker,
    image: &str,
    index: usize,
    execution_id: &str,
    tar_bytes: &[u8],
    binary_names: &[String],
    env: &[String],
) -> Result<u16, Error> {
    use bollard::models::{HostConfig, PortBinding, PortMap};

    let container_name = format!("objectiveai-{execution_id}-{index}");
    let options = CreateContainerOptionsBuilder::default()
        .name(container_name.as_str())
        .build();

    let mut port_bindings = PortMap::new();
    port_bindings.insert(
        MCP_CONTAINER_PORT.to_string(),
        Some(vec![PortBinding {
            host_ip: Some("0.0.0.0".to_string()),
            host_port: Some(String::new()),
        }]),
    );

    let config = ContainerCreateBody {
        image: Some(image.to_string()),
        cmd: Some(vec!["sleep".to_string(), "infinity".to_string()]),
        exposed_ports: Some(vec![MCP_CONTAINER_PORT.to_string()]),
        host_config: Some(HostConfig {
            port_bindings: Some(port_bindings),
            ..Default::default()
        }),
        ..Default::default()
    };

    let container = docker
        .create_container(Some(options), config)
        .await
        ?;

    docker
        .start_container(&container.id, None)
        .await
        ?;

    // Poll for port binding with exponential backoff
    let host_port = {
        let mut attempt = 0u32;
        loop {
            let delay = std::time::Duration::from_millis(10 * (1 << attempt.min(4)));
            tokio::time::sleep(delay).await;

            let inspect = docker
                .inspect_container(&container.id, None)
                .await
                ?;

            let port = inspect
                .network_settings
                .and_then(|ns| ns.ports)
                .and_then(|ports| ports.get(MCP_CONTAINER_PORT).cloned())
                .flatten()
                .and_then(|bindings| bindings.into_iter().next())
                .and_then(|b| b.host_port)
                .and_then(|p| p.parse::<u16>().ok());

            if let Some(p) = port {
                break p;
            }

            attempt += 1;
            if attempt > 10 {
                return Err(Error::PortTimeout(
                    format!("timeout after {attempt} attempts: failed to get host port for container {container_name}"),
                ));
            }
        }
    };

    // Upload binaries
    let upload_options = UploadToContainerOptionsBuilder::default()
        .path("/")
        .build();

    docker
        .upload_to_container(
            &container.id,
            Some(upload_options),
            bollard::body_full(tar_bytes.to_vec().into()),
        )
        .await
        ?;

    // Execute each binary
    let env_vec: Vec<&str> = env.iter().map(|s| s.as_str()).collect();
    for binary_name in binary_names {
        let exec_options = CreateExecOptions {
            cmd: Some(vec![binary_name.as_str()]),
            env: Some(env_vec.clone()),
            ..Default::default()
        };

        let exec = docker
            .create_exec(&container.id, exec_options)
            .await
            ?;

        let _ = docker
            .start_exec(
                &exec.id,
                Some(bollard::exec::StartExecOptions { detach: true, ..Default::default() }),
            )
            .await
            ?;
    }

    Ok(host_port)
}
