use std::sync::Arc;

use anyhow::Context;
use app_config::{AppConfig, AppEnv};
use axum::Router;
use tower_http::{catch_panic::CatchPanicLayer, services::ServeDir};

use crate::{
    auth::sessions::init_session, state::AppState, storage::init_storage,
    template_engine::init_templates,
};

pub async fn mkapp(conf: &AppConfig) -> Result<Router, anyhow::Error> {
    // FIXME customize 404
    let (state, pool) = init_state(conf).await?;
    let auth_layer = init_session(&pool, &state.db)
        .await
        .context("Failed to initialize session store")?;

    let router = crate::router::init_router()
        .with_state(state)
        .layer(auth_layer)
        .nest_service("/_assets", ServeDir::new("assets/dist"))
        // TODO: Propagate error message in AppEnv::Dev
        .layer(CatchPanicLayer::new());
    Ok(router)
}

pub async fn init_state(conf: &AppConfig) -> Result<(AppState, sqlx::SqlitePool), anyhow::Error> {
    let template_engine = init_templates();
    let (pool, db) = init_db(conf).await?;
    let storage = init_storage(conf).await?;
    let state = AppState {
        template_engine: Arc::new(template_engine),
        db,
        storage,
        dev: conf.env == AppEnv::Dev,
    };
    Ok((state, pool))
}

pub async fn init_db(
    conf: &AppConfig,
) -> Result<(sqlx::SqlitePool, sea_orm::DatabaseConnection), anyhow::Error> {
    let db_path = &conf.database_file;
    // https://cj.rs/blog/sqlite-pragma-cheatsheet-for-performance-and-consistency/
    let opts = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(db_path)
        .journal_mode(sqlx::sqlite::SqliteJournalMode::Wal)
        .synchronous(sqlx::sqlite::SqliteSynchronous::Normal)
        .foreign_keys(true);
    let pool = sqlx::SqlitePool::connect_with(opts)
        .await
        .context("Failed to connect to database")?;
    let db = sea_orm::SqlxSqliteConnector::from_sqlx_sqlite_pool(pool.clone());
    Ok((pool, db))
}
