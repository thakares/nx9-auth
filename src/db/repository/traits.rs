use crate::db::models::{
    ApiToken, Application, ApplicationMember, AuditFilter, AuditLog, GlobalSlug, Group, Permission,
    RefreshToken, Role, ServiceAccount, Session, Tenant, User, UserProfile,
};

#[async_trait::async_trait]
pub trait GlobalSlugsRepository: Send + Sync {
    async fn find_by_slug(&self, slug: &str) -> Result<Option<GlobalSlug>, sqlx::Error>;
}

#[async_trait::async_trait]
pub trait UsersRepository: Send + Sync {
    async fn find_by_id(&self, id: &str) -> Result<Option<User>, sqlx::Error>;
    async fn find_by_username(&self, username: &str) -> Result<Option<User>, sqlx::Error>;
    async fn list(&self, tenant_id: &str) -> Result<Vec<User>, sqlx::Error>;
    async fn create(
        &self,
        id: &str,
        tenant_id: &str,
        username: &str,
        password_hash: &str,
    ) -> Result<User, sqlx::Error>;
    async fn update_status(&self, id: &str, status: i32) -> Result<(), sqlx::Error>;
    async fn reassign_user_tenant_with_audit(
        &self,
        user_id: &str,
        destination_tenant_id: &str,
        actor_id: Option<&str>,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
    ) -> Result<(), sqlx::Error>;
    async fn update_password_hash(&self, id: &str, password_hash: &str) -> Result<(), sqlx::Error>;
    async fn set_last_login(&self, id: &str) -> Result<(), sqlx::Error>;
    async fn username_exists(&self, tenant_id: &str, username: &str) -> Result<bool, sqlx::Error>;
    async fn count_admins(&self) -> Result<i64, sqlx::Error>;
    async fn count(&self, tenant_id: &str) -> Result<i64, sqlx::Error>;
    async fn count_by_status(&self, tenant_id: &str, status: i32) -> Result<i64, sqlx::Error>;
    async fn get_profile(&self, user_id: &str) -> Result<Option<UserProfile>, sqlx::Error>;
    async fn upsert_profile(
        &self,
        user_id: &str,
        email: Option<&str>,
        full_name: Option<&str>,
    ) -> Result<UserProfile, sqlx::Error>;
}

#[async_trait::async_trait]
pub trait ServiceAccountsRepository: Send + Sync {
    async fn create(
        &self,
        id: &str,
        tenant_id: &str,
        name: &str,
        description: Option<&str>,
    ) -> Result<ServiceAccount, sqlx::Error>;
    async fn find_by_id(&self, id: &str) -> Result<Option<ServiceAccount>, sqlx::Error>;
    async fn list(&self, tenant_id: &str) -> Result<Vec<ServiceAccount>, sqlx::Error>;
    async fn set_enabled(&self, id: &str, enabled: bool) -> Result<(), sqlx::Error>;
    async fn delete(&self, id: &str) -> Result<(), sqlx::Error>;
    async fn count(&self, tenant_id: &str) -> Result<i64, sqlx::Error>;
}

#[async_trait::async_trait]
pub trait RolesRepository: Send + Sync {
    async fn list_all(&self) -> Result<Vec<Role>, sqlx::Error>;
    async fn find_by_name(&self, name: &str) -> Result<Option<Role>, sqlx::Error>;
    async fn find_by_id(&self, id: &str) -> Result<Option<Role>, sqlx::Error>;
    async fn list_for_user(&self, user_id: &str) -> Result<Vec<Role>, sqlx::Error>;
    async fn assign_to_user(&self, user_id: &str, role_id: &str) -> Result<(), sqlx::Error>;
    async fn remove_from_user(&self, user_id: &str, role_id: &str) -> Result<(), sqlx::Error>;
    async fn admin_role_exists(&self) -> Result<bool, sqlx::Error>;
    async fn create(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
    ) -> Result<Role, sqlx::Error>;
    async fn update(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
    ) -> Result<(), sqlx::Error>;
    async fn delete(&self, id: &str) -> Result<(), sqlx::Error>;
    async fn list_user_ids_for_role(&self, role_id: &str) -> Result<Vec<String>, sqlx::Error>;
}

#[async_trait::async_trait]
pub trait SessionsRepository: Send + Sync {
    async fn create(
        &self,
        id: &str,
        user_id: &str,
        token_hash: &str,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
        expires_at: &str,
    ) -> Result<Session, sqlx::Error>;
    async fn find_by_token_hash(&self, token_hash: &str) -> Result<Option<Session>, sqlx::Error>;
    async fn revoke(&self, id: &str) -> Result<(), sqlx::Error>;
    async fn revoke_all_for_user(&self, user_id: &str) -> Result<(), sqlx::Error>;
    async fn update_last_seen(&self, id: &str) -> Result<(), sqlx::Error>;
    async fn list_active_for_user(&self, user_id: &str) -> Result<Vec<Session>, sqlx::Error>;
    async fn list_all_active(&self) -> Result<Vec<Session>, sqlx::Error>;
    async fn count_active(&self) -> Result<i64, sqlx::Error>;
    async fn cleanup_expired(&self) -> Result<u64, sqlx::Error>;
    async fn revoke_others(&self, user_id: &str, except_id: &str) -> Result<u64, sqlx::Error>;
}

#[async_trait::async_trait]
pub trait ApplicationsRepository: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    async fn create_with_audit(
        &self,
        id: &str,
        tenant_id: &str,
        name: &str,
        slug: &str,
        client_id: &str,
        client_secret_hash: Option<&str>,
        description: Option<&str>,
        redirect_uris: Option<&str>,
        scopes: Option<&str>,
        audit_event: Option<crate::audit::AuditEvent<'_>>,
    ) -> Result<Application, sqlx::Error>;
    async fn find_by_slug(&self, slug: &str) -> Result<Option<Application>, sqlx::Error>;
    async fn find_by_client_id(&self, client_id: &str) -> Result<Option<Application>, sqlx::Error>;
    async fn find_by_id(&self, id: &str) -> Result<Option<Application>, sqlx::Error>;
    async fn list(&self, tenant_id: &str) -> Result<Vec<Application>, sqlx::Error>;
    async fn set_enabled(&self, id: &str, enabled: bool) -> Result<(), sqlx::Error>;
    async fn update_secret_hash(&self, id: &str, secret_hash: &str) -> Result<(), sqlx::Error>;
    async fn rotate_secret_with_audit(
        &self,
        id: &str,
        secret_hash: &str,
        audit_event: Option<crate::audit::AuditEvent<'_>>,
    ) -> Result<(), sqlx::Error>;
    #[allow(clippy::too_many_arguments)]
    async fn update(
        &self,
        id: &str,
        name: &str,
        slug: &str,
        description: Option<&str>,
        redirect_uris: Option<&str>,
        scopes: Option<&str>,
        enabled: bool,
    ) -> Result<(), sqlx::Error>;
    async fn delete(&self, id: &str) -> Result<(), sqlx::Error>;
    async fn count(&self, tenant_id: &str) -> Result<i64, sqlx::Error>;
}

#[async_trait::async_trait]
pub trait RefreshTokensRepository: Send + Sync {
    async fn create(
        &self,
        id: &str,
        user_id: &str,
        token_hash: &str,
        expires_at: &str,
    ) -> Result<RefreshToken, sqlx::Error>;
    async fn find_by_hash(&self, token_hash: &str) -> Result<Option<RefreshToken>, sqlx::Error>;
    async fn revoke(&self, id: &str) -> Result<(), sqlx::Error>;
    async fn revoke_all_for_user(&self, user_id: &str) -> Result<(), sqlx::Error>;
}

#[async_trait::async_trait]
pub trait AuditRepository: Send + Sync {
    #[allow(clippy::too_many_arguments)]
    async fn insert(
        &self,
        id: &str,
        actor_user_id: Option<&str>,
        target_user_id: Option<&str>,
        action: &str,
        resource_type: &str,
        resource_id: Option<&str>,
        severity: &str,
        ip_address: Option<&str>,
        user_agent: Option<&str>,
        metadata_json: Option<&str>,
    ) -> Result<AuditLog, sqlx::Error>;
    async fn list_recent(&self, limit: i64) -> Result<Vec<AuditLog>, sqlx::Error>;
    async fn count(&self) -> Result<i64, sqlx::Error>;
    async fn list_filtered(&self, filter: &AuditFilter) -> Result<Vec<AuditLog>, sqlx::Error>;
    async fn count_filtered(&self, filter: &AuditFilter) -> Result<i64, sqlx::Error>;
}

#[async_trait::async_trait]
pub trait AuditRepositoryExt: Send + Sync {
    async fn log(&self, event: crate::audit::AuditEvent<'_>) -> Result<AuditLog, sqlx::Error>;
}

#[async_trait::async_trait]
impl<T: ?Sized + AuditRepository> AuditRepositoryExt for T {
    async fn log(&self, event: crate::audit::AuditEvent<'_>) -> Result<AuditLog, sqlx::Error> {
        self.insert(
            &uuid::Uuid::new_v4().to_string(),
            event.actor_id,
            event.target_id,
            event.action,
            event.resource_type,
            event.resource_id,
            event.severity.as_str(),
            event.ip,
            event.ua,
            event.metadata,
        )
        .await
    }
}

#[async_trait::async_trait]
pub trait TokensRepository: Send + Sync {
    async fn create(
        &self,
        id: &str,
        user_id: &str,
        name: &str,
        token_hash: &str,
        expires_at: Option<&str>,
    ) -> Result<ApiToken, sqlx::Error>;
    async fn find_by_hash(&self, token_hash: &str) -> Result<Option<ApiToken>, sqlx::Error>;
    async fn list_for_user(&self, user_id: &str) -> Result<Vec<ApiToken>, sqlx::Error>;
    async fn find_by_id(&self, id: &str) -> Result<Option<ApiToken>, sqlx::Error>;
    async fn revoke(&self, id: &str) -> Result<(), sqlx::Error>;
    async fn update_last_used(&self, id: &str) -> Result<(), sqlx::Error>;
}

#[async_trait::async_trait]
pub trait PermissionsRepository: Send + Sync {
    async fn list_all(&self) -> Result<Vec<Permission>, sqlx::Error>;
    async fn list_for_role(&self, role_id: &str) -> Result<Vec<Permission>, sqlx::Error>;
    async fn assign_to_role(&self, role_id: &str, permission_id: &str) -> Result<(), sqlx::Error>;
    async fn remove_from_role(&self, role_id: &str, permission_id: &str)
    -> Result<(), sqlx::Error>;
    async fn clear_for_role(&self, role_id: &str) -> Result<(), sqlx::Error>;
    async fn find_by_name(&self, name: &str) -> Result<Option<Permission>, sqlx::Error>;
    async fn find_by_id(&self, id: &str) -> Result<Option<Permission>, sqlx::Error>;
    async fn list_for_user(&self, user_id: &str) -> Result<Vec<String>, sqlx::Error>;
    async fn user_has_permission(
        &self,
        user_id: &str,
        permission_name: &str,
    ) -> Result<bool, sqlx::Error>;
}

#[async_trait::async_trait]
pub trait TenantsRepository: Send + Sync {
    async fn find_by_id(&self, id: &str) -> Result<Option<Tenant>, sqlx::Error>;
    async fn find_by_slug(&self, slug: &str) -> Result<Option<Tenant>, sqlx::Error>;
    async fn list(&self) -> Result<Vec<Tenant>, sqlx::Error>;
    async fn create(&self, id: &str, name: &str, slug: Option<&str>)
    -> Result<Tenant, sqlx::Error>;
    async fn update(&self, id: &str, name: &str, slug: Option<&str>) -> Result<(), sqlx::Error>;
    async fn set_enabled(&self, id: &str, enabled: bool) -> Result<(), sqlx::Error>;
    async fn delete(&self, id: &str) -> Result<(), sqlx::Error>;
}

#[async_trait::async_trait]
pub trait GroupsRepository: Send + Sync {
    async fn list(&self, tenant_id: &str) -> Result<Vec<Group>, sqlx::Error>;
    async fn find_by_id(&self, id: &str) -> Result<Option<Group>, sqlx::Error>;
    async fn create(
        &self,
        id: &str,
        tenant_id: &str,
        name: &str,
        description: Option<&str>,
    ) -> Result<Group, sqlx::Error>;
    async fn update(
        &self,
        id: &str,
        name: &str,
        description: Option<&str>,
    ) -> Result<(), sqlx::Error>;
    async fn delete(&self, id: &str) -> Result<(), sqlx::Error>;
    async fn count_members(&self, group_id: &str) -> Result<i64, sqlx::Error>;
    async fn list_members(
        &self,
        group_id: &str,
    ) -> Result<Vec<crate::db::models::User>, sqlx::Error>;
    async fn add_member(&self, group_id: &str, user_id: &str) -> Result<(), sqlx::Error>;
    async fn remove_member(&self, group_id: &str, user_id: &str) -> Result<(), sqlx::Error>;
    async fn count(&self, tenant_id: &str) -> Result<i64, sqlx::Error>;
}

/// Application membership repository (user ↔ application assignment).
///
/// Membership roles are lightweight metadata and do not modify global RBAC.
#[async_trait::async_trait]
pub trait ApplicationMembersRepository: Send + Sync {
    async fn list_by_application(
        &self,
        application_id: &str,
    ) -> Result<Vec<ApplicationMember>, sqlx::Error>;
    async fn list_by_user(&self, user_id: &str) -> Result<Vec<ApplicationMember>, sqlx::Error>;
    async fn find(
        &self,
        application_id: &str,
        user_id: &str,
    ) -> Result<Option<ApplicationMember>, sqlx::Error>;
    async fn add(
        &self,
        id: &str,
        application_id: &str,
        user_id: &str,
        role: &str,
    ) -> Result<ApplicationMember, sqlx::Error>;
    async fn update_role(
        &self,
        application_id: &str,
        user_id: &str,
        role: &str,
    ) -> Result<(), sqlx::Error>;
    async fn set_enabled(
        &self,
        application_id: &str,
        user_id: &str,
        enabled: bool,
    ) -> Result<(), sqlx::Error>;
    async fn remove(&self, application_id: &str, user_id: &str) -> Result<(), sqlx::Error>;

    async fn add_with_audit(
        &self,
        id: &str,
        application_id: &str,
        user_id: &str,
        role: &str,
        audit_event: Option<crate::audit::AuditEvent<'_>>,
    ) -> Result<ApplicationMember, sqlx::Error>;
    async fn update_role_with_audit(
        &self,
        application_id: &str,
        user_id: &str,
        role: &str,
        audit_event: Option<crate::audit::AuditEvent<'_>>,
    ) -> Result<(), sqlx::Error>;
    async fn set_enabled_with_audit(
        &self,
        application_id: &str,
        user_id: &str,
        enabled: bool,
        audit_event: Option<crate::audit::AuditEvent<'_>>,
    ) -> Result<(), sqlx::Error>;
    async fn remove_with_audit(
        &self,
        application_id: &str,
        user_id: &str,
        audit_event: Option<crate::audit::AuditEvent<'_>>,
    ) -> Result<(), sqlx::Error>;
}
