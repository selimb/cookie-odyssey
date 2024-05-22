use anyhow::Context as _;
use axum::async_trait;
use axum_login::AuthzBackend as _;
use sea_orm::{ColumnTrait as _, QueryFilter as _, Select};
use std::collections::HashSet;

use super::sessions::AuthBackend;
use crate::AuthSession;

/// Simplest permissioning ever.
#[derive(Debug, Clone, Eq, PartialEq, Hash)]
pub enum Permission {
    Admin,
}

#[async_trait]
impl axum_login::AuthzBackend for AuthBackend {
    type Permission = Permission;

    async fn get_user_permissions(
        &self,
        user: &Self::User,
    ) -> Result<HashSet<Self::Permission>, Self::Error> {
        let mut perms = HashSet::new();
        if user.0.admin {
            perms.insert(Permission::Admin);
        }
        Ok(perms)
    }
}

impl AuthBackend {
    pub async fn filter_journal_entries(
        &self,
        session: &AuthSession,
        mut q: Select<entities::journal_entry::Entity>,
    ) -> Result<Select<entities::journal_entry::Entity>, anyhow::Error> {
        let user = session.user.as_ref().expect("Should have a user");
        let can_view_draft = self
            .has_perm(user, Permission::Admin)
            .await
            .context("has_perm failed")?;

        if !can_view_draft {
            q = q.filter(entities::journal_entry::Column::Draft.eq(false));
        }
        Ok(q)
    }
}
