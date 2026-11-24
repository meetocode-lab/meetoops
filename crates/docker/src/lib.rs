use bollard::container::{
    InspectContainerOptions, ListContainersOptions, LogOutput, LogsOptions, RemoveContainerOptions,
    RestartContainerOptions, StartContainerOptions, StatsOptions, StopContainerOptions,
};
use bollard::exec::{CreateExecOptions, ResizeExecOptions, StartExecOptions, StartExecResults};
use bollard::image::{
    CreateImageOptions, ListImagesOptions, PruneImagesOptions, RemoveImageOptions,
};
use bollard::network::{CreateNetworkOptions, ListNetworksOptions, PruneNetworksOptions};
use bollard::volume::{
    CreateVolumeOptions, ListVolumesOptions, PruneVolumesOptions, RemoveVolumeOptions,
};
use bollard::Docker;
use futures_util::StreamExt;
use meetoops_common::mask_env_pair;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;
use tokio::io::AsyncWriteExt;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Error)]
pub enum DockerError {
    #[error("docker engine unavailable: {0}")]
    Unavailable(String),
    #[error("invalid input: {0}")]
    Validation(String),
    #[error("docker api error: {0}")]
    Api(#[from] bollard::errors::Error),
}

/// Real probe result from a local Docker Engine.
#[derive(Debug, Clone, Serialize)]
pub struct LocalDockerProbe {
    pub available: bool,
    pub endpoint: String,
    pub docker_version: Option<String>,
    pub api_version: Option<String>,
    pub operating_system: Option<String>,
    pub architecture: Option<String>,
    pub containers_total: Option<i64>,
    pub containers_running: Option<i64>,
    pub containers_stopped: Option<i64>,
    pub images_total: Option<i64>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContainerPortDto {
    pub ip: Option<String>,
    pub private_port: u16,
    pub public_port: Option<u16>,
    pub protocol: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContainerSummaryDto {
    pub id: String,
    pub short_id: String,
    pub name: String,
    pub image: String,
    pub state: String,
    pub status: String,
    pub created: Option<i64>,
    pub ports: Vec<ContainerPortDto>,
    pub networks: Vec<String>,
    pub labels: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct EnvVarDto {
    pub key: String,
    pub value: String,
    pub is_secret: bool,
    pub masked: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct MountDto {
    pub mount_type: String,
    pub source: String,
    pub destination: String,
    pub mode: String,
    pub rw: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ContainerInspectDto {
    pub id: String,
    pub short_id: String,
    pub name: String,
    pub image: String,
    pub state: String,
    pub running: bool,
    pub paused: bool,
    pub restarting: bool,
    pub restart_count: Option<i64>,
    pub health_status: Option<String>,
    pub created: Option<String>,
    pub started_at: Option<String>,
    pub command: Vec<String>,
    pub env: Vec<EnvVarDto>,
    pub mounts: Vec<MountDto>,
    pub networks: Vec<String>,
    pub labels: HashMap<String, String>,
    pub restart_policy: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ImageSummaryDto {
    pub id: String,
    pub short_id: String,
    pub repo_tags: Vec<String>,
    pub size_bytes: i64,
    pub created: i64,
    pub containers: i64,
}

#[derive(Debug, Clone, Serialize)]
pub struct VolumeSummaryDto {
    pub name: String,
    pub driver: String,
    pub mountpoint: String,
    pub created_at: Option<String>,
    pub scope: String,
    pub used_by: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct NetworkSummaryDto {
    pub id: String,
    pub short_id: String,
    pub name: String,
    pub driver: String,
    pub scope: String,
    pub internal: bool,
    pub attachable: bool,
    pub subnet: Option<String>,
    pub gateway: Option<String>,
    pub connected_containers: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogLineDto {
    pub stream: String,
    pub line: String,
}

pub fn default_local_endpoint() -> String {
    if let Ok(host) = std::env::var("MEETOOPS_DOCKER_HOST") {
        if !host.trim().is_empty() {
            return host;
        }
    }
    if cfg!(windows) {
        "//./pipe/docker_engine".to_string()
    } else {
        "/var/run/docker.sock".to_string()
    }
}

fn connect_local() -> Result<Docker, DockerError> {
    Docker::connect_with_local_defaults().map_err(|e| DockerError::Unavailable(e.to_string()))
}

pub async fn probe_local_docker() -> LocalDockerProbe {
    let endpoint = default_local_endpoint();
    let client = match connect_local() {
        Ok(c) => c,
        Err(err) => {
            return LocalDockerProbe {
                available: false,
                endpoint,
                docker_version: None,
                api_version: None,
                operating_system: None,
                architecture: None,
                containers_total: None,
                containers_running: None,
                containers_stopped: None,
                images_total: None,
                error_message: Some(err.to_string()),
            };
        }
    };

    match client.info().await {
        Ok(info) => {
            let version = client.version().await.ok();
            LocalDockerProbe {
                available: true,
                endpoint,
                docker_version: info
                    .server_version
                    .or_else(|| version.as_ref().and_then(|v| v.version.clone())),
                api_version: version.and_then(|v| v.api_version),
                operating_system: info.operating_system,
                architecture: info.architecture,
                containers_total: info.containers,
                containers_running: info.containers_running,
                containers_stopped: info.containers_stopped,
                images_total: info.images,
                error_message: None,
            }
        }
        Err(err) => LocalDockerProbe {
            available: false,
            endpoint,
            docker_version: None,
            api_version: None,
            operating_system: None,
            architecture: None,
            containers_total: None,
            containers_running: None,
            containers_stopped: None,
            images_total: None,
            error_message: Some(err.to_string()),
        },
    }
}

pub async fn list_containers(all: bool) -> Result<Vec<ContainerSummaryDto>, DockerError> {
    let client = connect_local()?;
    let list = client
        .list_containers(Some(ListContainersOptions::<String> {
            all,
            ..Default::default()
        }))
        .await?;

    let mut out = Vec::with_capacity(list.len());
    for c in list {
        let id = c.id.unwrap_or_default();
        let short_id: String = id.chars().take(12).collect();
        let name = c
            .names
            .and_then(|n| n.into_iter().next())
            .map(|n| n.trim_start_matches('/').to_string())
            .unwrap_or_else(|| short_id.clone());

        let ports = c
            .ports
            .unwrap_or_default()
            .into_iter()
            .map(|p| ContainerPortDto {
                ip: p.ip,
                private_port: p.private_port,
                public_port: p.public_port,
                protocol: p
                    .typ
                    .map(|t| t.to_string())
                    .unwrap_or_else(|| "tcp".to_string()),
            })
            .collect();

        let networks = c
            .network_settings
            .and_then(|ns| ns.networks)
            .map(|m| m.into_keys().collect())
            .unwrap_or_default();

        out.push(ContainerSummaryDto {
            id,
            short_id,
            name,
            image: c.image.unwrap_or_default(),
            state: c.state.unwrap_or_else(|| "unknown".to_string()),
            status: c.status.unwrap_or_default(),
            created: c.created,
            ports,
            networks,
            labels: c.labels.unwrap_or_default(),
        });
    }
    Ok(out)
}

pub async fn inspect_container(
    id: &str,
    reveal_secrets: bool,
) -> Result<ContainerInspectDto, DockerError> {
    let client = connect_local()?;
    let info = client
        .inspect_container(id, Some(InspectContainerOptions { size: false }))
        .await?;

    let full_id = info.id.unwrap_or_else(|| id.to_string());
    let short_id = full_id.chars().take(12).collect::<String>();
    let name = info
        .name
        .map(|n| n.trim_start_matches('/').to_string())
        .unwrap_or_else(|| short_id.clone());

    let state = info.state.unwrap_or_default();
    let config = info.config.unwrap_or_default();
    let host_config = info.host_config.unwrap_or_default();

    let env = config
        .env
        .unwrap_or_default()
        .into_iter()
        .map(|entry| {
            let (k, v) = entry.split_once('=').unwrap_or((entry.as_str(), ""));
            if reveal_secrets {
                let (_, is_secret) = mask_env_pair(k, v);
                EnvVarDto {
                    key: k.to_string(),
                    value: v.to_string(),
                    is_secret,
                    masked: false,
                }
            } else {
                let (masked_val, is_secret) = mask_env_pair(k, v);
                EnvVarDto {
                    key: k.to_string(),
                    value: masked_val,
                    is_secret,
                    masked: is_secret,
                }
            }
        })
        .collect();

    let mounts = info
        .mounts
        .unwrap_or_default()
        .into_iter()
        .map(|m| MountDto {
            mount_type: m
                .typ
                .map(|t| t.to_string())
                .unwrap_or_else(|| "bind".to_string()),
            source: m.source.unwrap_or_default(),
            destination: m.destination.unwrap_or_default(),
            mode: m.mode.unwrap_or_default(),
            rw: m.rw.unwrap_or(true),
        })
        .collect();

    let networks = info
        .network_settings
        .and_then(|ns| ns.networks)
        .map(|m| m.into_keys().collect())
        .unwrap_or_default();

    let health_status = state
        .health
        .and_then(|h| h.status)
        .map(|s| s.to_string());

    let restart_policy = host_config
        .restart_policy
        .and_then(|rp| rp.name)
        .map(|n| n.to_string());

    Ok(ContainerInspectDto {
        id: full_id,
        short_id,
        name,
        image: config.image.unwrap_or_default(),
        state: state
            .status
            .map(|s| s.to_string())
            .unwrap_or_else(|| "unknown".to_string()),
        running: state.running.unwrap_or(false),
        paused: state.paused.unwrap_or(false),
        restarting: state.restarting.unwrap_or(false),
        restart_count: info.restart_count,
        health_status,
        created: info.created,
        started_at: state.started_at,
        command: config.cmd.unwrap_or_default(),
        env,
        mounts,
        networks,
        labels: config.labels.unwrap_or_default(),
        restart_policy,
    })
}

pub async fn container_action(id: &str, action: &str) -> Result<(), DockerError> {
    let client = connect_local()?;
    match action {
        "start" => {
            client
                .start_container(id, None::<StartContainerOptions<String>>)
                .await?;
        }
        "stop" => {
            client
                .stop_container(id, Some(StopContainerOptions { t: 10 }))
                .await?;
        }
        "restart" => {
            client
                .restart_container(id, Some(RestartContainerOptions { t: 10 }))
                .await?;
        }
        "pause" => {
            client.pause_container(id).await?;
        }
        "unpause" => {
            client.unpause_container(id).await?;
        }
        other => {
            return Err(DockerError::Validation(format!(
                "unsupported container action: {other}"
            )));
        }
    }
    Ok(())
}

pub async fn remove_container(id: &str, force: bool, remove_volumes: bool) -> Result<(), DockerError> {
    let client = connect_local()?;
    client
        .remove_container(
            id,
            Some(RemoveContainerOptions {
                force,
                v: remove_volumes,
                ..Default::default()
            }),
        )
        .await?;
    Ok(())
}

pub async fn container_logs_snapshot(
    id: &str,
    tail: usize,
    timestamps: bool,
) -> Result<Vec<LogLineDto>, DockerError> {
    let client = connect_local()?;
    let tail_str = tail.clamp(1, 2000).to_string();
    let mut stream = client.logs(
        id,
        Some(LogsOptions::<String> {
            stdout: true,
            stderr: true,
            follow: false,
            timestamps,
            tail: tail_str,
            ..Default::default()
        }),
    );

    let mut lines = Vec::new();
    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(out) => lines.push(map_log_output(out)),
            Err(err) => return Err(DockerError::Api(err)),
        }
    }
    Ok(lines)
}

/// Streams live container logs over a bounded `mpsc::Sender` until cancelled or EOF.
pub async fn stream_container_logs(
    id: String,
    tail: usize,
    timestamps: bool,
    tx: mpsc::Sender<LogLineDto>,
    cancel: CancellationToken,
) -> Result<(), DockerError> {
    let client = connect_local()?;
    let tail_str = tail.clamp(1, 1000).to_string();
    let mut stream = client.logs(
        &id,
        Some(LogsOptions::<String> {
            stdout: true,
            stderr: true,
            follow: true,
            timestamps,
            tail: tail_str,
            ..Default::default()
        }),
    );

    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            item = stream.next() => {
                match item {
                    Some(Ok(out)) => {
                        if tx.send(map_log_output(out)).await.is_err() {
                            break;
                        }
                    }
                    Some(Err(err)) => return Err(DockerError::Api(err)),
                    None => break,
                }
            }
        }
    }
    Ok(())
}

fn map_log_output(out: LogOutput) -> LogLineDto {
    match out {
        LogOutput::StdOut { message } => LogLineDto {
            stream: "stdout".to_string(),
            line: String::from_utf8_lossy(&message).trim_end_matches('\n').to_string(),
        },
        LogOutput::StdErr { message } => LogLineDto {
            stream: "stderr".to_string(),
            line: String::from_utf8_lossy(&message).trim_end_matches('\n').to_string(),
        },
        LogOutput::Console { message } => LogLineDto {
            stream: "console".to_string(),
            line: String::from_utf8_lossy(&message).trim_end_matches('\n').to_string(),
        },
        LogOutput::StdIn { message } => LogLineDto {
            stream: "stdin".to_string(),
            line: String::from_utf8_lossy(&message).trim_end_matches('\n').to_string(),
        },
    }
}

pub async fn list_images() -> Result<Vec<ImageSummaryDto>, DockerError> {
    let client = connect_local()?;
    let images = client
        .list_images(Some(ListImagesOptions::<String> {
            all: false,
            ..Default::default()
        }))
        .await?;

    Ok(images
        .into_iter()
        .map(|img| {
            let short_id = img
                .id
                .trim_start_matches("sha256:")
                .chars()
                .take(12)
                .collect();
            ImageSummaryDto {
                id: img.id,
                short_id,
                repo_tags: img.repo_tags,
                size_bytes: img.size,
                created: img.created,
                containers: img.containers,
            }
        })
        .collect())
}

pub async fn pull_image(image_ref: &str) -> Result<Vec<String>, DockerError> {
    let trimmed = image_ref.trim();
    if trimmed.is_empty() || trimmed.contains(' ') {
        return Err(DockerError::Validation(
            "Invalid image reference.".to_string(),
        ));
    }
    let client = connect_local()?;
    let mut stream = client.create_image(
        Some(CreateImageOptions {
            from_image: trimmed,
            ..Default::default()
        }),
        None,
        None,
    );

    let mut statuses = Vec::new();
    while let Some(item) = stream.next().await {
        match item {
            Ok(info) => {
                if let Some(status) = info.status {
                    statuses.push(status);
                }
            }
            Err(err) => return Err(DockerError::Api(err)),
        }
    }
    Ok(statuses)
}

pub async fn remove_image(id: &str, force: bool) -> Result<(), DockerError> {
    let client = connect_local()?;
    client
        .remove_image(
            id,
            Some(RemoveImageOptions {
                force,
                noprune: false,
            }),
            None,
        )
        .await?;
    Ok(())
}

pub async fn list_volumes() -> Result<Vec<VolumeSummaryDto>, DockerError> {
    let client = connect_local()?;
    let resp = client
        .list_volumes(Some(ListVolumesOptions::<String>::default()))
        .await?;

    // Cross-reference mounted containers
    let containers = client
        .list_containers(Some(ListContainersOptions::<String> {
            all: true,
            ..Default::default()
        }))
        .await
        .unwrap_or_default();

    let mut usage: HashMap<String, Vec<String>> = HashMap::new();
    for c in containers {
        let cname = c
            .names
            .and_then(|n| n.into_iter().next())
            .map(|n| n.trim_start_matches('/').to_string())
            .unwrap_or_else(|| c.id.unwrap_or_default());
        for m in c.mounts.unwrap_or_default() {
            if let Some(vname) = m.name {
                usage.entry(vname).or_default().push(cname.clone());
            }
        }
    }

    Ok(resp
        .volumes
        .unwrap_or_default()
        .into_iter()
        .map(|v| {
            let used_by = usage.remove(&v.name).unwrap_or_default();
            VolumeSummaryDto {
                name: v.name,
                driver: v.driver,
                mountpoint: v.mountpoint,
                created_at: v.created_at,
                scope: v
                    .scope
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| "local".to_string()),
                used_by,
            }
        })
        .collect())
}

pub async fn create_volume(name: &str, driver: Option<&str>) -> Result<VolumeSummaryDto, DockerError> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.contains(' ') {
        return Err(DockerError::Validation("Invalid volume name.".to_string()));
    }
    let client = connect_local()?;
    let v = client
        .create_volume(CreateVolumeOptions {
            name: trimmed.to_string(),
            driver: driver.unwrap_or("local").to_string(),
            ..Default::default()
        })
        .await?;

    Ok(VolumeSummaryDto {
        name: v.name,
        driver: v.driver,
        mountpoint: v.mountpoint,
        created_at: v.created_at,
        scope: v
            .scope
            .map(|s| s.to_string())
            .unwrap_or_else(|| "local".to_string()),
        used_by: vec![],
    })
}

pub async fn remove_volume(name: &str, force: bool) -> Result<(), DockerError> {
    let client = connect_local()?;
    client
        .remove_volume(name, Some(RemoveVolumeOptions { force }))
        .await?;
    Ok(())
}

pub async fn list_networks() -> Result<Vec<NetworkSummaryDto>, DockerError> {
    let client = connect_local()?;
    let nets = client
        .list_networks(Some(ListNetworksOptions::<String>::default()))
        .await?;

    Ok(nets
        .into_iter()
        .map(|n| {
            let id = n.id.unwrap_or_default();
            let short_id = id.chars().take(12).collect();
            let ipam_cfg = n
                .ipam
                .and_then(|ipam| ipam.config)
                .and_then(|cfgs| cfgs.into_iter().next());
            let connected_containers = n
                .containers
                .unwrap_or_default()
                .into_values()
                .filter_map(|c| c.name)
                .collect();

            NetworkSummaryDto {
                id,
                short_id,
                name: n.name.unwrap_or_default(),
                driver: n.driver.unwrap_or_default(),
                scope: n.scope.unwrap_or_default(),
                internal: n.internal.unwrap_or(false),
                attachable: n.attachable.unwrap_or(false),
                subnet: ipam_cfg.as_ref().and_then(|c| c.subnet.clone()),
                gateway: ipam_cfg.as_ref().and_then(|c| c.gateway.clone()),
                connected_containers,
            }
        })
        .collect())
}

pub async fn create_network(name: &str, driver: Option<&str>) -> Result<String, DockerError> {
    let trimmed = name.trim();
    if trimmed.is_empty() || trimmed.contains(' ') {
        return Err(DockerError::Validation("Invalid network name.".to_string()));
    }
    let client = connect_local()?;
    let res = client
        .create_network(CreateNetworkOptions {
            name: trimmed.to_string(),
            driver: driver.unwrap_or("bridge").to_string(),
            check_duplicate: true,
            ..Default::default()
        })
        .await?;
    Ok(res.id)
}

pub async fn remove_network(id: &str) -> Result<(), DockerError> {
    let client = connect_local()?;
    client.remove_network(id).await?;
    Ok(())
}

// --- Phase 4: Docker Compose Stacks & Interactive PTY Exec ---

#[derive(Debug, Clone, Serialize)]
pub struct ComposeServiceItemDto {
    pub container_id: String,
    pub short_id: String,
    pub container_name: String,
    pub service_name: String,
    pub image: String,
    pub state: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ComposeStackDto {
    pub project_name: String,
    pub working_dir: Option<String>,
    pub config_files: Option<String>,
    pub total_services: usize,
    pub running_services: usize,
    pub services: Vec<ComposeServiceItemDto>,
}

/// Detects Docker Compose stacks from real `com.docker.compose.project` container labels.
pub async fn list_compose_stacks() -> Result<Vec<ComposeStackDto>, DockerError> {
    let containers = list_containers(true).await?;
    let mut grouped: HashMap<String, ComposeStackDto> = HashMap::new();

    for c in containers {
        let Some(project) = c.labels.get("com.docker.compose.project").cloned() else {
            continue;
        };
        let service_name = c
            .labels
            .get("com.docker.compose.service")
            .cloned()
            .unwrap_or_else(|| c.name.clone());
        let working_dir = c.labels.get("com.docker.compose.project.working_dir").cloned();
        let config_files = c.labels.get("com.docker.compose.project.config_files").cloned();

        let entry = grouped.entry(project.clone()).or_insert_with(|| ComposeStackDto {
            project_name: project,
            working_dir,
            config_files,
            total_services: 0,
            running_services: 0,
            services: Vec::new(),
        });

        entry.total_services += 1;
        if c.state == "running" {
            entry.running_services += 1;
        }
        entry.services.push(ComposeServiceItemDto {
            container_id: c.id,
            short_id: c.short_id,
            container_name: c.name,
            service_name,
            image: c.image,
            state: c.state,
            status: c.status,
        });
    }

    let mut stacks: Vec<ComposeStackDto> = grouped.into_values().collect();
    stacks.sort_by(|a, b| a.project_name.cmp(&b.project_name));
    Ok(stacks)
}

pub async fn stack_action(project_name: &str, action: &str) -> Result<usize, DockerError> {
    if !matches!(action, "start" | "stop" | "restart") {
        return Err(DockerError::Validation(format!(
            "unsupported stack action: {action}"
        )));
    }
    let stacks = list_compose_stacks().await?;
    let Some(stack) = stacks.into_iter().find(|s| s.project_name == project_name) else {
        return Err(DockerError::Validation(format!(
            "Compose stack '{project_name}' not found."
        )));
    };
    let mut affected = 0usize;
    for svc in stack.services {
        container_action(&svc.container_id, action).await?;
        affected += 1;
    }
    Ok(affected)
}

#[derive(Debug)]
pub enum ExecInputMsg {
    Data(Vec<u8>),
    Resize { cols: u16, rows: u16 },
}

/// Runs an interactive TTY `docker exec` session bridged via bounded channels.
pub async fn run_container_exec(
    container_id: String,
    shell: String,
    mut stdin_rx: mpsc::Receiver<ExecInputMsg>,
    stdout_tx: mpsc::Sender<Vec<u8>>,
    cancel: CancellationToken,
) -> Result<(), DockerError> {
    let safe_shell = match shell.as_str() {
        "/bin/bash" | "/bin/sh" | "/bin/ash" | "sh" | "bash" => shell,
        _ => "/bin/sh".to_string(),
    };

    let client = connect_local()?;
    let exec = client
        .create_exec(
            &container_id,
            CreateExecOptions {
                attach_stdin: Some(true),
                attach_stdout: Some(true),
                attach_stderr: Some(true),
                tty: Some(true),
                cmd: Some(vec![safe_shell]),
                ..Default::default()
            },
        )
        .await?;

    let exec_id = exec.id;
    let started = client
        .start_exec(
            &exec_id,
            Some(StartExecOptions {
                detach: false,
                tty: true,
                output_capacity: None,
            }),
        )
        .await?;

    let StartExecResults::Attached {
        mut output,
        mut input,
    } = started
    else {
        return Err(DockerError::Unavailable(
            "exec session detached unexpectedly".to_string(),
        ));
    };

    let cancel_out = cancel.clone();
    let out_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                _ = cancel_out.cancelled() => break,
                chunk = output.next() => {
                    match chunk {
                        Some(Ok(msg)) => {
                            let bytes = msg.into_bytes().to_vec();
                            if stdout_tx.send(bytes).await.is_err() {
                                break;
                            }
                        }
                        _ => break,
                    }
                }
            }
        }
    });

    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            msg = stdin_rx.recv() => {
                match msg {
                    Some(ExecInputMsg::Data(bytes)) => {
                        if input.write_all(&bytes).await.is_err() {
                            break;
                        }
                        let _ = input.flush().await;
                    }
                    Some(ExecInputMsg::Resize { cols, rows }) => {
                        let _ = client.resize_exec(&exec_id, ResizeExecOptions { height: rows, width: cols }).await;
                    }
                    None => break,
                }
            }
        }
    }

    cancel.cancel();
    out_task.abort();
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
pub struct ContainerStatsSnapshotDto {
    pub cpu_percent: Option<f64>,
    pub memory_used_bytes: Option<u64>,
    pub memory_limit_bytes: Option<u64>,
    pub memory_percent: Option<f64>,
    pub net_rx_bytes: Option<u64>,
    pub net_tx_bytes: Option<u64>,
    pub pids: Option<u64>,
}

/// Fetches a single real-time resource usage snapshot for a running container via Docker Stats API.
pub async fn container_stats_snapshot(id: &str) -> Result<ContainerStatsSnapshotDto, DockerError> {
    let client = connect_local()?;
    let mut stream = client.stats(
        id,
        Some(StatsOptions {
            stream: false,
            one_shot: true,
        }),
    );

    let Some(Ok(stats)) = stream.next().await else {
        return Ok(ContainerStatsSnapshotDto {
            cpu_percent: None,
            memory_used_bytes: None,
            memory_limit_bytes: None,
            memory_percent: None,
            net_rx_bytes: None,
            net_tx_bytes: None,
            pids: None,
        });
    };

    let cpu_delta = stats
        .cpu_stats
        .cpu_usage
        .total_usage
        .saturating_sub(stats.precpu_stats.cpu_usage.total_usage) as f64;
    let sys_delta = stats
        .cpu_stats
        .system_cpu_usage
        .unwrap_or(0)
        .saturating_sub(stats.precpu_stats.system_cpu_usage.unwrap_or(0)) as f64;
    let online_cpus = stats.cpu_stats.online_cpus.unwrap_or(1).max(1) as f64;
    let cpu_percent = if sys_delta > 0.0 && cpu_delta >= 0.0 {
        Some(((cpu_delta / sys_delta) * online_cpus * 100.0 * 10.0).round() / 10.0)
    } else {
        None
    };

    let mem_usage = stats.memory_stats.usage;
    let mem_limit = stats.memory_stats.limit;
    let memory_percent = match (mem_usage, mem_limit) {
        (Some(u), Some(l)) if l > 0 => Some(((u as f64 / l as f64) * 1000.0).round() / 10.0),
        _ => None,
    };

    let (net_rx_bytes, net_tx_bytes) = if let Some(nets) = stats.networks {
        let mut rx = 0u64;
        let mut tx = 0u64;
        for n in nets.values() {
            rx = rx.saturating_add(n.rx_bytes);
            tx = tx.saturating_add(n.tx_bytes);
        }
        (Some(rx), Some(tx))
    } else {
        (None, None)
    };

    Ok(ContainerStatsSnapshotDto {
        cpu_percent,
        memory_used_bytes: mem_usage,
        memory_limit_bytes: mem_limit,
        memory_percent,
        net_rx_bytes,
        net_tx_bytes,
        pids: stats.pids_stats.current,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct PruneResultDto {
    pub deleted_count: usize,
    pub space_reclaimed_bytes: u64,
}

pub async fn prune_images() -> Result<PruneResultDto, DockerError> {
    let client = connect_local()?;
    let res = client
        .prune_images(Some(PruneImagesOptions::<String>::default()))
        .await?;
    Ok(PruneResultDto {
        deleted_count: res.images_deleted.map(|v| v.len()).unwrap_or(0),
        space_reclaimed_bytes: res.space_reclaimed.unwrap_or(0).max(0) as u64,
    })
}

pub async fn prune_volumes() -> Result<PruneResultDto, DockerError> {
    let client = connect_local()?;
    let res = client
        .prune_volumes(Some(PruneVolumesOptions::<String>::default()))
        .await?;
    Ok(PruneResultDto {
        deleted_count: res.volumes_deleted.map(|v| v.len()).unwrap_or(0),
        space_reclaimed_bytes: res.space_reclaimed.unwrap_or(0).max(0) as u64,
    })
}

pub async fn prune_networks() -> Result<PruneResultDto, DockerError> {
    let client = connect_local()?;
    let res = client
        .prune_networks(Some(PruneNetworksOptions::<String>::default()))
        .await?;
    Ok(PruneResultDto {
        deleted_count: res.networks_deleted.map(|v| v.len()).unwrap_or(0),
        space_reclaimed_bytes: 0,
    })
}
