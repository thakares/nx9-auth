use crate::db::models::AuditSeverity;

/// A structured audit event to be persisted and logged.
#[derive(Debug)]
pub struct AuditEvent<'a> {
    /// The user performing the action (None for system/CLI events).
    pub actor_id: Option<&'a str>,
    /// The user being acted upon, if applicable.
    pub target_id: Option<&'a str>,
    /// Machine-readable action name (e.g. `"login_success"`, `"user_created"`).
    pub action: &'a str,
    /// Resource category (e.g. `"user"`, `"session"`, `"token"`).
    pub resource_type: &'a str,
    /// Specific resource ID, if applicable.
    pub resource_id: Option<&'a str>,
    /// Event severity.
    pub severity: AuditSeverity,
    /// Client IP address.
    pub ip: Option<&'a str>,
    /// Client User-Agent string.
    pub ua: Option<&'a str>,
    /// Optional structured metadata (serialized JSON string).
    pub metadata: Option<&'a str>,
}

impl<'a> AuditEvent<'a> {
    /// Convenience constructor for info-level system events with no actor/IP.
    pub fn system(action: &'a str, resource_type: &'a str) -> Self {
        Self {
            actor_id: None,
            target_id: None,
            action,
            resource_type,
            resource_id: None,
            severity: AuditSeverity::Info,
            ip: None,
            ua: None,
            metadata: None,
        }
    }
}
