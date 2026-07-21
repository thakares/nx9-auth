use crate::db::models::{Group, User};
use crate::db::repository::traits::GroupsRepository;
use async_trait::async_trait;
use sqlx::PgPool;

pub struct PostgresGroupsRepository {
    pub pool: PgPool,
}

#[async_trait]
impl GroupsRepository for PostgresGroupsRepository {
    async fn list(&self, _tenant_id: &str) -> Result<Vec<Group>, sqlx::Error> {
        unimplemented!()
    }

    async fn find_by_id(&self, _id: &str) -> Result<Option<Group>, sqlx::Error> {
        unimplemented!()
    }

    async fn create(
        &self,
        _id: &str,
        _tenant_id: &str,
        _name: &str,
        _description: Option<&str>,
    ) -> Result<Group, sqlx::Error> {
        unimplemented!()
    }

    async fn update(
        &self,
        _id: &str,
        _name: &str,
        _description: Option<&str>,
    ) -> Result<(), sqlx::Error> {
        unimplemented!()
    }

    async fn delete(&self, _id: &str) -> Result<(), sqlx::Error> {
        unimplemented!()
    }

    async fn count_members(&self, _group_id: &str) -> Result<i64, sqlx::Error> {
        unimplemented!()
    }

    async fn list_members(&self, _group_id: &str) -> Result<Vec<User>, sqlx::Error> {
        unimplemented!()
    }

    async fn add_member(&self, _group_id: &str, _user_id: &str) -> Result<(), sqlx::Error> {
        unimplemented!()
    }

    async fn remove_member(&self, _group_id: &str, _user_id: &str) -> Result<(), sqlx::Error> {
        unimplemented!()
    }

    async fn count(&self, _tenant_id: &str) -> Result<i64, sqlx::Error> {
        unimplemented!()
    }
}
