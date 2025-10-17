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
    video_transcoding::{
        backend::{
            github_action::GithubActionVideoTranscoder, in_process::InProcessVideoTranscoder,
            traits::VideoTranscodingBackend,
        },
        daemon::VideoTranscodeDaemon,
    },
};

pub async fn mkapp(state: AppState, pool: &sqlx::SqlitePool) -> Result<Router, anyhow::Error> {
    // FIXME customize 404
    let auth_layer = init_session(pool, &state.db)
        .await
        .context("Failed to initialize session store")?;

    let router = crate::router::init_router(state.clone())
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

    let video_transcoder = init_video_transcoder(conf, &db, storage.clone()).await?;

    let state = AppState {
        github_client_token: conf.video_transcoding.github_client_token.clone(),
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
    conf: &AppConfig,
    db: &sea_orm::DatabaseConnection,
    storage: Arc<FileStore>,
) -> anyhow::Result<VideoTranscodeDaemon> {
    let backend: Box<dyn VideoTranscodingBackend> = match conf.video_transcoding.in_process {
        true => {
            let work_dir = temp_dir().join("cookie-odyssey-video-transcode");
            tokio::fs::create_dir_all(&work_dir)
                .await
                .context("Failed to create work directory for video transcoder")?;

            let backend = InProcessVideoTranscoder {
                db: db.clone(),
                storage,
                work_dir: work_dir.to_string_lossy().to_string(),
            };
            Box::new(backend)
        }
        false => {
            let github_token = conf
                .video_transcoding
                .github_token
                .clone()
                .context("Github token is required")?;
            let github_workflow_url = conf
                .video_transcoding
                .github_url
                .clone()
                .context("Github URL is required")?;

            let backend = GithubActionVideoTranscoder {
                db: db.clone(),
                storage,
                github_workflow_url,
                github_token,
                server_name: conf.server_name.clone(),
                reqwest: reqwest::Client::new(),
            };
            Box::new(backend)
        }
    };

    let video_transcoder = VideoTranscodeDaemon::start(backend, db.clone()).await;
    Ok(video_transcoder)
}
