use meetoops_core::{BootstrapConfig, CoreStore, HostMetricPoint};
use meetoops_security::{MasterKeyVault, RateLimiter};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;

#[derive(Clone)]
pub struct AppState {
    pub store: CoreStore,
    pub vault: Arc<MasterKeyVault>,
    pub config: BootstrapConfig,
    pub active_port: u16,
    pub auth_rate_limiter: Arc<RateLimiter>,
    pub setup_rate_limiter: Arc<RateLimiter>,
    pub metrics_tx: broadcast::Sender<HostMetricPoint>,
}

impl AppState {
    pub fn new(
        store: CoreStore,
        vault: MasterKeyVault,
        config: BootstrapConfig,
        active_port: u16,
    ) -> Self {
        let (metrics_tx, _) = broadcast::channel(128);
        Self {
            store,
            vault: Arc::new(vault),
            config,
            active_port,
            auth_rate_limiter: Arc::new(RateLimiter::new(15, Duration::from_secs(60))),
            setup_rate_limiter: Arc::new(RateLimiter::new(10, Duration::from_secs(60))),
            metrics_tx,
        }
    }
}
