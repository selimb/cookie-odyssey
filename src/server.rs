use std::sync::Arc;

use anyhow::Context;
use app_config::AppConfig;
use axum::{response::Html, Router};
use tera::Tera;

use crate::{
    auth::sessions::init_session,
    storage::{init_storage, FileStore},
    template_engine::init_templates,
};

#[derive(Clone)]
pub struct AppState {
    pub tera: Arc<Tera>,
    pub db: sea_orm::DatabaseConnection,
    pub storage: Arc<FileStore>,
}

impl AppState {
    pub fn render(
        &self,
        template_name: &str,
        context: &tera::Context,
    ) -> Result<Html<String>, tera::Error> {
        let s = self.tera.render(template_name, context)?;
        Ok(Html(s))
    }
}

pub async fn mkapp(conf: &AppConfig) -> Result<Router, anyhow::Error> {
    // FIXME customize 404
    let (state, pool) = init_state(conf).await?;
    let auth_layer = init_session(&pool, &state.db)
        .await
        .context("Failed to initialize session store")?;

    let router = crate::router::init_router()
        .with_state(state)
        .layer(auth_layer);
    Ok(router)
}

async fn init_state(conf: &AppConfig) -> Result<(AppState, sqlx::SqlitePool), anyhow::Error> {
    let tera = init_templates()?;
    let (pool, db) = init_db(conf).await?;
    let storage = init_storage(conf).context("Failed to initialize file storage")?;
    let state = AppState {
        tera: Arc::new(tera),
        db,
        storage,
    };
    Ok((state, pool))
}

pub async fn init_db(
    conf: &AppConfig,
) -> Result<(sqlx::SqlitePool, sea_orm::DatabaseConnection), anyhow::Error> {
    let db_url = conf.database_url();
    let pool = sqlx::SqlitePool::connect(&db_url)
        .await
        .context("Failed to connect to database")?;
    let db = sea_orm::SqlxSqliteConnector::from_sqlx_sqlite_pool(pool.clone());
    Ok((pool, db))
}
