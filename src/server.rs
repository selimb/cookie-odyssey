use std::{env::temp_dir, sync::Arc};

use anyhow::Context;
use app_config::{AppConfig, AppEnv};
use axum::Router;
use tower_http::{catch_panic::CatchPanicLayer, services::ServeDir};

use crate::{
    auth::sessions::init_session,
    state::AppState,
    storage::{init_storage, FileStore},
    template_engine::init_templates,
    video_transcoding::{daemon::VideoTranscodeDaemon, manager::VideoTranscoder},
};

pub async fn mkapp(state: AppState, pool: &sqlx::SqlitePool) -> Result<Router, anyhow::Error> {
    // FIXME customize 404
    let auth_layer = init_session(pool, &state.db)
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

    let video_transcoder: Option<VideoTranscodeDaemon> = if conf.video_transcoding.in_process {
        Some(init_video_transcoder(&db, storage.clone()).await?)
    } else {
        None
    };

    let state = AppState {
        template_engine: Arc::new(template_engine),
        db,
        storage,
        video_transcoder: Arc::new(video_transcoder),
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

async fn init_video_transcoder(
    db: &sea_orm::DatabaseConnection,
    storage: Arc<FileStore>,
) -> anyhow::Result<VideoTranscodeDaemon> {
    let work_dir = temp_dir().join("cookie-odyssey-video-transcode");
    tokio::fs::create_dir_all(&work_dir)
        .await
        .context("Failed to create work directory for video transcoder")?;

    let video_transcoder = VideoTranscodeDaemon::start(VideoTranscoder {
        db: db.clone(),
        storage: storage.clone(),
        work_dir: work_dir.to_string_lossy().to_string(),
    })
    .await;
    Ok(video_transcoder)
}
