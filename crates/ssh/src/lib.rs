use async_trait::async_trait;
use russh::client;
use russh::keys::decode_secret_key;
use russh::ChannelMsg;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::Duration;
use thiserror::Error;
use tokio::sync::mpsc;
use tokio_util::sync::CancellationToken;

#[derive(Debug, Error)]
pub enum SshError {
    #[error("validation error: {0}")]
    Validation(String),
    #[error("ssh connection failed: {0}")]
    Connect(String),
    #[error("ssh authentication failed: {0}")]
    Auth(String),
    #[error("ssh operation failed: {0}")]
    Protocol(String),
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum SshAuth {
    Password {
        password: String,
    },
    PrivateKey {
        private_key_pem: String,
        passphrase: Option<String>,
    },
}

impl std::fmt::Debug for SshAuth {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Password { .. } => f.write_str("SshAuth::Password([REDACTED])"),
            Self::PrivateKey { .. } => f.write_str("SshAuth::PrivateKey([REDACTED])"),
        }
    }
}

#[derive(Clone)]
pub struct SshConnectionConfig {
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth: SshAuth,
}

#[derive(Debug, Clone, Serialize)]
pub struct SshExecOutput {
    pub exit_code: u32,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug)]
pub enum SshInputMsg {
    Data(Vec<u8>),
    Resize { cols: u32, rows: u32 },
}

struct ClientHandler;

#[async_trait]
impl client::Handler for ClientHandler {
    type Error = russh::Error;

    async fn check_server_key(
        &mut self,
        _server_public_key: &russh::keys::ssh_key::PublicKey,
    ) -> Result<bool, Self::Error> {
        Ok(true)
    }
}

async fn connect_and_auth(
    cfg: &SshConnectionConfig,
) -> Result<client::Handle<ClientHandler>, SshError> {
    let host = cfg.host.trim();
    if host.is_empty() {
        return Err(SshError::Validation("SSH host is required.".to_string()));
    }
    if cfg.username.trim().is_empty() {
        return Err(SshError::Validation(
            "SSH username is required.".to_string(),
        ));
    }

    let config = Arc::new(client::Config {
        inactivity_timeout: Some(Duration::from_secs(1800)),
        keepalive_interval: Some(Duration::from_secs(20)),
        keepalive_max: 3,
        ..Default::default()
    });

    let addr = format!("{}:{}", host, cfg.port);
    let mut handle = tokio::time::timeout(
        Duration::from_secs(12),
        client::connect(config, addr.as_str(), ClientHandler),
    )
    .await
    .map_err(|_| SshError::Connect(format!("Timed out connecting to {addr}")))?
    .map_err(|e| SshError::Connect(e.to_string()))?;

    let authed = match &cfg.auth {
        SshAuth::Password { password } => handle
            .authenticate_password(cfg.username.trim(), password)
            .await
            .map_err(|e| SshError::Auth(e.to_string()))?,
        SshAuth::PrivateKey {
            private_key_pem,
            passphrase,
        } => {
            let key = decode_secret_key(private_key_pem, passphrase.as_deref())
                .map_err(|e| SshError::Auth(format!("Invalid private key: {e}")))?;
            let key_with_alg =
                russh::keys::key::PrivateKeyWithHashAlg::new(Arc::new(key), None)
                    .map_err(|e| SshError::Auth(e.to_string()))?;
            handle
                .authenticate_publickey(cfg.username.trim(), key_with_alg)
                .await
                .map_err(|e| SshError::Auth(e.to_string()))?
        }
    };

    if !authed {
        return Err(SshError::Auth(
            "SSH authentication rejected by remote host.".to_string(),
        ));
    }

    Ok(handle)
}

/// Verifies SSH connectivity & authentication and returns OS + user banner.
pub async fn test_connection(cfg: &SshConnectionConfig) -> Result<String, SshError> {
    let out = exec_command(cfg, "uname -srm && whoami", None, Duration::from_secs(15)).await?;
    let banner = out
        .stdout
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty())
        .collect::<Vec<_>>()
        .join(" · ");
    Ok(if banner.is_empty() {
        "SSH connected".to_string()
    } else {
        banner
    })
}

/// Runs a non-interactive SSH command and captures stdout, stderr, and exit code.
pub async fn exec_command(
    cfg: &SshConnectionConfig,
    command: &str,
    stdin_data: Option<&[u8]>,
    timeout: Duration,
) -> Result<SshExecOutput, SshError> {
    let handle = connect_and_auth(cfg).await?;
    let mut channel = handle
        .channel_open_session()
        .await
        .map_err(|e| SshError::Protocol(e.to_string()))?;

    channel
        .exec(true, command)
        .await
        .map_err(|e| SshError::Protocol(e.to_string()))?;

    if let Some(data) = stdin_data {
        channel
            .data(data)
            .await
            .map_err(|e| SshError::Protocol(e.to_string()))?;
        channel
            .eof()
            .await
            .map_err(|e| SshError::Protocol(e.to_string()))?;
    }

    let mut stdout = Vec::new();
    let mut stderr = Vec::new();
    let mut exit_code: u32 = 0;

    let read_fut = async {
        while let Some(msg) = channel.wait().await {
            match msg {
                ChannelMsg::Data { data } => stdout.extend_from_slice(&data),
                ChannelMsg::ExtendedData { data, ext } => {
                    if ext == 1 {
                        stderr.extend_from_slice(&data);
                    }
                }
                ChannelMsg::ExitStatus { exit_status } => {
                    exit_code = exit_status;
                }
                ChannelMsg::Eof | ChannelMsg::Close => break,
                _ => {}
            }
        }
    };

    tokio::time::timeout(timeout, read_fut)
        .await
        .map_err(|_| SshError::Protocol("SSH command timed out.".to_string()))?;

    let _ = handle
        .disconnect(russh::Disconnect::ByApplication, "", "en")
        .await;

    Ok(SshExecOutput {
        exit_code,
        stdout: String::from_utf8_lossy(&stdout).to_string(),
        stderr: String::from_utf8_lossy(&stderr).to_string(),
    })
}

/// Runs a full interactive `xterm-256color` PTY shell over SSH bridged to bounded channels.
pub async fn run_interactive_pty(
    cfg: SshConnectionConfig,
    cols: u32,
    rows: u32,
    mut stdin_rx: mpsc::Receiver<SshInputMsg>,
    stdout_tx: mpsc::Sender<Vec<u8>>,
    cancel: CancellationToken,
) -> Result<(), SshError> {
    let handle = connect_and_auth(&cfg).await?;
    let mut channel = handle
        .channel_open_session()
        .await
        .map_err(|e| SshError::Protocol(e.to_string()))?;

    channel
        .request_pty(false, "xterm-256color", cols.max(40), rows.max(12), 0, 0, &[])
        .await
        .map_err(|e| SshError::Protocol(e.to_string()))?;

    channel
        .request_shell(false)
        .await
        .map_err(|e| SshError::Protocol(e.to_string()))?;

    loop {
        tokio::select! {
            _ = cancel.cancelled() => break,
            in_msg = stdin_rx.recv() => {
                match in_msg {
                    Some(SshInputMsg::Data(bytes)) => {
                        if channel.data(&bytes[..]).await.is_err() {
                            break;
                        }
                    }
                    Some(SshInputMsg::Resize { cols, rows }) => {
                        let _ = channel.window_change(cols.max(20), rows.max(5), 0, 0).await;
                    }
                    None => break,
                }
            }
            out_msg = channel.wait() => {
                match out_msg {
                    Some(ChannelMsg::Data { data }) => {
                        if stdout_tx.send(data.to_vec()).await.is_err() {
                            break;
                        }
                    }
                    Some(ChannelMsg::ExtendedData { data, .. }) => {
                        if stdout_tx.send(data.to_vec()).await.is_err() {
                            break;
                        }
                    }
                    Some(ChannelMsg::Eof | ChannelMsg::Close) | None => break,
                    _ => {}
                }
            }
        }
    }

    let _ = channel.close().await;
    let _ = handle
        .disconnect(russh::Disconnect::ByApplication, "", "en")
        .await;
    Ok(())
}

/// Sanitizes any MeetoOps username or email into a safe POSIX Linux username (`a-z0-9_-`).
pub fn sanitize_linux_username(raw: &str) -> String {
    let base = raw.split('@').next().unwrap_or(raw).to_ascii_lowercase();
    let mut out = String::new();
    for ch in base.chars() {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-' {
            out.push(ch);
        } else if ch == ' ' || ch == '.' {
            out.push('_');
        }
    }
    let trimmed = out.trim_matches(|c| c == '-' || c == '_');
    let candidate = if trimmed.is_empty()
        || !trimmed
            .chars()
            .next()
            .map(|c| c.is_ascii_lowercase() || c == '_')
            .unwrap_or(false)
    {
        format!("u_{trimmed}")
    } else {
        trimmed.to_string()
    };
    candidate.chars().take(31).collect()
}

pub fn validate_linux_username(name: &str) -> Result<(), SshError> {
    if name.len() < 2 || name.len() > 32 {
        return Err(SshError::Validation(
            "Linux username must be 2–32 characters.".to_string(),
        ));
    }
    let first = name.chars().next().unwrap_or('0');
    if !(first.is_ascii_lowercase() || first == '_') {
        return Err(SshError::Validation(
            "Linux username must start with a lowercase letter or underscore.".to_string(),
        ));
    }
    if !name
        .chars()
        .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_' || c == '-')
    {
        return Err(SshError::Validation(
            "Linux username may only contain lowercase letters, digits, '_', or '-'.".to_string(),
        ));
    }
    if matches!(name, "root" | "daemon" | "bin" | "sys" | "nobody") {
        return Err(SshError::Validation(
            "Cannot provision a reserved system account.".to_string(),
        ));
    }
    Ok(())
}

/// Provisions a non-root Linux user on the target host with `sudo` (password-protected, once per session)
/// and optional `docker` group membership. Uses stdin for `chpasswd` so the password never appears in `ps`.
pub async fn provision_linux_sudo_user(
    admin_cfg: &SshConnectionConfig,
    new_username: &str,
    new_password: &str,
    add_docker_group: bool,
) -> Result<String, SshError> {
    validate_linux_username(new_username)?;
    if new_password.len() < 8 {
        return Err(SshError::Validation(
            "Linux user password must be at least 8 characters.".to_string(),
        ));
    }
    if new_password.contains('\n') || new_password.contains('\r') {
        return Err(SshError::Validation(
            "Password must not contain newline characters.".to_string(),
        ));
    }

    let docker_step = if add_docker_group {
        format!("if getent group docker >/dev/null 2>&1; then usermod -aG docker {new_username}; fi")
    } else {
        "true".to_string()
    };

    let script = format!(
        "set -e; \
         if ! id -u {u} >/dev/null 2>&1; then useradd -m -s /bin/bash {u}; fi; \
         chpasswd; \
         if getent group sudo >/dev/null 2>&1; then usermod -aG sudo {u}; \
         elif getent group wheel >/dev/null 2>&1; then usermod -aG wheel {u}; fi; \
         {docker_step}; \
         id {u}",
        u = new_username
    );

    let stdin_line = format!("{new_username}:{new_password}\n");
    let out = exec_command(
        admin_cfg,
        &script,
        Some(stdin_line.as_bytes()),
        Duration::from_secs(20),
    )
    .await?;

    if out.exit_code != 0 {
        return Err(SshError::Protocol(format!(
            "Provisioning failed (exit {}): {}",
            out.exit_code,
            out.stderr.trim()
        )));
    }

    Ok(out.stdout.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sanitizes_and_validates_linux_usernames() {
        assert_eq!(sanitize_linux_username("Admin Osce"), "admin_osce");
        assert_eq!(sanitize_linux_username("davin.metoti@gmail.com"), "davin_metoti");
        assert!(validate_linux_username("admin_osce").is_ok());
        assert!(validate_linux_username("root").is_err());
        assert!(validate_linux_username("bad user!").is_err());
    }
}
