use std::collections::HashSet;

use axum::async_trait;

use super::sessions::AuthBackend;

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
