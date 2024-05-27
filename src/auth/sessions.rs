//! Mostly copied/adapted from https://github.com/maxcountryman/axum-login/blob/main/examples/sqlite/src/web/app.rs
use anyhow::Context;
use app_config::AppEnv;
use axum::async_trait;
use axum_login::tower_sessions::ExpiredDeletion;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

const DELETE_EXPIRED_INTERVAL: chrono::Duration = chrono::Duration::hours(1);
const COOKIE_MAX_AGE: tower_sessions::cookie::time::Duration =
    tower_sessions::cookie::time::Duration::days(365);

pub async fn init_session(
    sqlite_pool: &sqlx::SqlitePool,
    db: &sea_orm::DatabaseConnection,
) -> Result<
    axum_login::AuthManagerLayer<AuthBackend, tower_sessions_sqlx_store::SqliteStore>,
    anyhow::Error,
> {
    let session_store = tower_sessions_sqlx_store::SqliteStore::new(sqlite_pool.clone());
    session_store
        .migrate()
        .await
        .context("Failed to apply migrations for session store")?;

    // TODO: Do we need to track and/or clean this up?
    let _deletion_task = tokio::task::spawn(
        session_store
            .clone()
            .continuously_delete_expired(DELETE_EXPIRED_INTERVAL.to_std().unwrap()),
    );

    // NOTE: Don't bother with encrypting cookies;
    let session_layer = tower_sessions::SessionManagerLayer::new(session_store)
        // SameSite::Strict is too strict: can't follow link from email!
        .with_same_site(tower_sessions::cookie::SameSite::Lax)
        .with_expiry(tower_sessions::Expiry::OnInactivity(COOKIE_MAX_AGE))
        .with_secure(!AppEnv::is_dev())
        .with_http_only(true);

    let auth_backend = AuthBackend { db: db.clone() };
    let auth_layer = axum_login::AuthManagerLayerBuilder::new(auth_backend, session_layer).build();

    Ok(auth_layer)
}

#[derive(Clone, Debug, Serialize)]
pub struct AuthUser(pub entities::user::Model);

impl axum_login::AuthUser for AuthUser {
    type Id = i32;

    fn id(&self) -> Self::Id {
        self.0.id
    }

    fn session_auth_hash(&self) -> &[u8] {
        // Use the password hash as the auth hash, which means the session is
        // invalidated when the user changes their password.
        self.0.password.as_bytes()
    }
}

// This allows us to extract the authentication fields from forms. We use this
// to authenticate requests with the backend.
#[derive(Debug, Clone, Deserialize)]
pub struct Credentials {
    pub email: String,
    pub password: String,
    pub next: String,
}

#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    #[error(transparent)]
    Db(#[from] sea_orm::DbErr),

    #[error(transparent)]
    TaskJoin(#[from] tokio::task::JoinError),

    #[error("approval is still pending")]
    PendingApproval,
}

#[derive(Debug, Clone)]
pub struct AuthBackend {
    db: sea_orm::DatabaseConnection,
}

impl AuthBackend {
    pub fn hash_password(password: String) -> String {
        password_auth::generate_hash(password)
    }

    pub fn normalize_email(email: impl AsRef<str>) -> String {
        email.as_ref().trim().to_lowercase()
    }
}

#[async_trait]
impl axum_login::AuthnBackend for AuthBackend {
    type User = AuthUser;
    type Credentials = Credentials;
    type Error = AuthError;

    async fn authenticate(
        &self,
        creds: Self::Credentials,
    ) -> Result<Option<Self::User>, Self::Error> {
        let user = entities::user::Entity::find()
            .filter(entities::user::Column::Email.eq(Self::normalize_email(&creds.email)))
            .one(&self.db)
            .await?;

        let user = match user {
            None => {
                return Ok(None);
            }
            Some(user) => {
                if user.approved {
                    AuthUser(user)
                } else {
                    return Err(AuthError::PendingApproval);
                }
            }
        };

        tokio::task::spawn_blocking(move || {
            match password_auth::verify_password(creds.password.trim(), &user.0.password) {
                Ok(_) => Ok(Some(user)),
                Err(_) => Ok(None),
            }
        })
        .await?
    }

    async fn get_user(
        &self,
        user_id: &axum_login::UserId<Self>,
    ) -> Result<Option<Self::User>, Self::Error> {
        let user = entities::user::Entity::find_by_id(*user_id)
            .one(&self.db)
            .await?;

        Ok(user.map(AuthUser))
    }
}

pub type AuthSession = axum_login::AuthSession<AuthBackend>;
