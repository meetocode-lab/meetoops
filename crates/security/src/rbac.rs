use meetoops_common::Role;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Permission {
    DashboardRead,
    MonitoringRead,
    DockerRead,
    DockerOperate,
    DockerDelete,
    DockerRevealSecrets,
    LogsRead,
    TerminalExec,
    DiscoveryRead,
    ServicesCredentials,
    DbRead,
    DbWrite,
    RedisRead,
    RedisWrite,
    RedisAdmin,
    ServersRead,
    ServersWrite,
    ServersDelete,
    AlertsRead,
    AlertsWrite,
    UsersRead,
    UsersWrite,
    UsersDelete,
    AuditRead,
    SettingsRead,
    SettingsWrite,
    BackupManage,
}

impl Permission {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::DashboardRead => "dashboard:read",
            Self::MonitoringRead => "monitoring:read",
            Self::DockerRead => "docker:read",
            Self::DockerOperate => "docker:operate",
            Self::DockerDelete => "docker:delete",
            Self::DockerRevealSecrets => "docker:reveal_secrets",
            Self::LogsRead => "logs:read",
            Self::TerminalExec => "terminal:exec",
            Self::DiscoveryRead => "discovery:read",
            Self::ServicesCredentials => "services:credentials",
            Self::DbRead => "db:read",
            Self::DbWrite => "db:write",
            Self::RedisRead => "redis:read",
            Self::RedisWrite => "redis:write",
            Self::RedisAdmin => "redis:admin",
            Self::ServersRead => "servers:read",
            Self::ServersWrite => "servers:write",
            Self::ServersDelete => "servers:delete",
            Self::AlertsRead => "alerts:read",
            Self::AlertsWrite => "alerts:write",
            Self::UsersRead => "users:read",
            Self::UsersWrite => "users:write",
            Self::UsersDelete => "users:delete",
            Self::AuditRead => "audit:read",
            Self::SettingsRead => "settings:read",
            Self::SettingsWrite => "settings:write",
            Self::BackupManage => "backup:manage",
        }
    }
}

pub fn has_permission(role: Role, perm: Permission) -> bool {
    match role {
        Role::SuperAdmin => true,
        Role::Admin => !matches!(perm, Permission::SettingsWrite | Permission::BackupManage),
        Role::Operator => matches!(
            perm,
            Permission::DashboardRead
                | Permission::MonitoringRead
                | Permission::DockerRead
                | Permission::DockerOperate
                | Permission::LogsRead
                | Permission::DiscoveryRead
                | Permission::DbRead
                | Permission::RedisRead
                | Permission::ServersRead
                | Permission::AlertsRead
                | Permission::SettingsRead
        ),
        Role::Viewer => matches!(
            perm,
            Permission::DashboardRead
                | Permission::MonitoringRead
                | Permission::DockerRead
                | Permission::LogsRead
                | Permission::DiscoveryRead
                | Permission::ServersRead
                | Permission::AlertsRead
        ),
    }
}

pub fn permissions_for_role(role: Role) -> Vec<&'static str> {
    let all = [
        Permission::DashboardRead,
        Permission::MonitoringRead,
        Permission::DockerRead,
        Permission::DockerOperate,
        Permission::DockerDelete,
        Permission::DockerRevealSecrets,
        Permission::LogsRead,
        Permission::TerminalExec,
        Permission::DiscoveryRead,
        Permission::ServicesCredentials,
        Permission::DbRead,
        Permission::DbWrite,
        Permission::RedisRead,
        Permission::RedisWrite,
        Permission::RedisAdmin,
        Permission::ServersRead,
        Permission::ServersWrite,
        Permission::ServersDelete,
        Permission::AlertsRead,
        Permission::AlertsWrite,
        Permission::UsersRead,
        Permission::UsersWrite,
        Permission::UsersDelete,
        Permission::AuditRead,
        Permission::SettingsRead,
        Permission::SettingsWrite,
        Permission::BackupManage,
    ];
    all.into_iter()
        .filter(|p| has_permission(role, *p))
        .map(|p| p.as_str())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn enforces_rbac_matrix() {
        assert!(has_permission(Role::SuperAdmin, Permission::SettingsWrite));
        assert!(!has_permission(Role::Admin, Permission::SettingsWrite));
        assert!(has_permission(Role::Admin, Permission::DockerDelete));
        assert!(has_permission(Role::Operator, Permission::DockerOperate));
        assert!(!has_permission(Role::Operator, Permission::DockerDelete));
        assert!(!has_permission(Role::Viewer, Permission::DockerOperate));
        assert!(has_permission(Role::Viewer, Permission::DashboardRead));
    }
}
