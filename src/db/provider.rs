#[cfg(feature = "postgres")]
use sqlx::PgPool;
use sqlx::SqlitePool;

use crate::db::repository::traits::*;

#[async_trait::async_trait]
pub trait DatabaseProvider: Send + Sync {
    fn users(&self) -> Box<dyn UsersRepository>;
    fn applications(&self) -> Box<dyn ApplicationsRepository>;
    fn audit(&self) -> Box<dyn AuditRepository>;
    fn permissions(&self) -> Box<dyn PermissionsRepository>;
    fn refresh_tokens(&self) -> Box<dyn RefreshTokensRepository>;
    fn roles(&self) -> Box<dyn RolesRepository>;
    fn service_accounts(&self) -> Box<dyn ServiceAccountsRepository>;
    fn sessions(&self) -> Box<dyn SessionsRepository>;
    fn tokens(&self) -> Box<dyn TokensRepository>;
    fn tenants(&self) -> Box<dyn TenantsRepository>;
    fn groups(&self) -> Box<dyn GroupsRepository>;
}

#[cfg(feature = "sqlite")]
pub struct SqliteProvider {
    pub pool: SqlitePool,
}

#[cfg(feature = "sqlite")]
impl SqliteProvider {
    pub fn new(pool: SqlitePool) -> Self {
        Self { pool }
    }
}

#[cfg(feature = "sqlite")]
#[async_trait::async_trait]
impl DatabaseProvider for SqliteProvider {
    fn users(&self) -> Box<dyn UsersRepository> {
        Box::new(
            crate::db::repository::sqlite::users::SqliteUsersRepository {
                pool: self.pool.clone(),
            },
        )
    }
    fn applications(&self) -> Box<dyn ApplicationsRepository> {
        Box::new(
            crate::db::repository::sqlite::applications::SqliteApplicationsRepository {
                pool: self.pool.clone(),
            },
        )
    }
    fn audit(&self) -> Box<dyn AuditRepository> {
        Box::new(
            crate::db::repository::sqlite::audit::SqliteAuditRepository {
                pool: self.pool.clone(),
            },
        )
    }
    fn permissions(&self) -> Box<dyn PermissionsRepository> {
        Box::new(
            crate::db::repository::sqlite::permissions::SqlitePermissionsRepository {
                pool: self.pool.clone(),
            },
        )
    }
    fn refresh_tokens(&self) -> Box<dyn RefreshTokensRepository> {
        Box::new(
            crate::db::repository::sqlite::refresh_tokens::SqliteRefreshTokensRepository {
                pool: self.pool.clone(),
            },
        )
    }
    fn roles(&self) -> Box<dyn RolesRepository> {
        Box::new(
            crate::db::repository::sqlite::roles::SqliteRolesRepository {
                pool: self.pool.clone(),
            },
        )
    }
    fn service_accounts(&self) -> Box<dyn ServiceAccountsRepository> {
        Box::new(
            crate::db::repository::sqlite::service_accounts::SqliteServiceAccountsRepository {
                pool: self.pool.clone(),
            },
        )
    }
    fn sessions(&self) -> Box<dyn SessionsRepository> {
        Box::new(
            crate::db::repository::sqlite::sessions::SqliteSessionsRepository {
                pool: self.pool.clone(),
            },
        )
    }
    fn tokens(&self) -> Box<dyn TokensRepository> {
        Box::new(
            crate::db::repository::sqlite::tokens::SqliteTokensRepository {
                pool: self.pool.clone(),
            },
        )
    }
    fn tenants(&self) -> Box<dyn TenantsRepository> {
        Box::new(
            crate::db::repository::sqlite::tenants::SqliteTenantsRepository {
                pool: self.pool.clone(),
            },
        )
    }
    fn groups(&self) -> Box<dyn GroupsRepository> {
        Box::new(
            crate::db::repository::sqlite::groups::SqliteGroupsRepository {
                pool: self.pool.clone(),
            },
        )
    }
}

#[cfg(feature = "postgres")]
pub struct PostgresProvider {
    pub pool: PgPool,
}

#[cfg(feature = "postgres")]
impl PostgresProvider {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[cfg(feature = "postgres")]
#[async_trait::async_trait]
impl DatabaseProvider for PostgresProvider {
    fn users(&self) -> Box<dyn UsersRepository> {
        Box::new(
            crate::db::repository::postgres::users::PostgresUsersRepository {
                pool: self.pool.clone(),
            },
        )
    }
    fn applications(&self) -> Box<dyn ApplicationsRepository> {
        Box::new(
            crate::db::repository::postgres::applications::PostgresApplicationsRepository {
                pool: self.pool.clone(),
            },
        )
    }
    fn audit(&self) -> Box<dyn AuditRepository> {
        Box::new(
            crate::db::repository::postgres::audit::PostgresAuditRepository {
                pool: self.pool.clone(),
            },
        )
    }
    fn permissions(&self) -> Box<dyn PermissionsRepository> {
        Box::new(
            crate::db::repository::postgres::permissions::PostgresPermissionsRepository {
                pool: self.pool.clone(),
            },
        )
    }
    fn refresh_tokens(&self) -> Box<dyn RefreshTokensRepository> {
        Box::new(
            crate::db::repository::postgres::refresh_tokens::PostgresRefreshTokensRepository {
                pool: self.pool.clone(),
            },
        )
    }
    fn roles(&self) -> Box<dyn RolesRepository> {
        Box::new(
            crate::db::repository::postgres::roles::PostgresRolesRepository {
                pool: self.pool.clone(),
            },
        )
    }
    fn service_accounts(&self) -> Box<dyn ServiceAccountsRepository> {
        Box::new(
            crate::db::repository::postgres::service_accounts::PostgresServiceAccountsRepository {
                pool: self.pool.clone(),
            },
        )
    }
    fn sessions(&self) -> Box<dyn SessionsRepository> {
        Box::new(
            crate::db::repository::postgres::sessions::PostgresSessionsRepository {
                pool: self.pool.clone(),
            },
        )
    }
    fn tokens(&self) -> Box<dyn TokensRepository> {
        Box::new(
            crate::db::repository::postgres::tokens::PostgresTokensRepository {
                pool: self.pool.clone(),
            },
        )
    }
    fn tenants(&self) -> Box<dyn TenantsRepository> {
        Box::new(
            crate::db::repository::postgres::tenants::PostgresTenantsRepository {
                pool: self.pool.clone(),
            },
        )
    }
    fn groups(&self) -> Box<dyn GroupsRepository> {
        Box::new(
            crate::db::repository::postgres::groups::PostgresGroupsRepository {
                pool: self.pool.clone(),
            },
        )
    }
}
