use std::{env::temp_dir, path::PathBuf, sync::Arc};

use anyhow::Context;
use app_config::{AppConfig, AppEnv};
use axum::Router;
use tower_http::{catch_panic::CatchPanicLayer, services::ServeDir};

use crate::{
    assets::AssetManifest,
    auth::sessions::init_session,
    comment::email_worker::CommentEmailWorker,
    email::EmailClient,
    notification::email_worker::NotificationEmailWorker,
    state::AppState,
    storage::{init_storage, FileStore},
    template_engine::{init_templates, templates_dir},
    video_transcoding::{
        backend::{
            github_action::GithubActionVideoTranscoder, in_process::InProcessVideoTranscoder,
            traits::VideoTranscodingBackend,
        },
        daemon::VideoTranscoder,
    },
};

const ASSETS_URL_BASE: &str = "/_assets";

/// Returns the directory that holds `templates` and `assets`.
///
/// In prod, this is the directory of the executable -- see [release-dir].
/// In dev, the executable is in `target/`, so the working directory is used.
pub fn release_dir() -> Result<PathBuf, anyhow::Error> {
    if AppEnv::is_dev() {
        return Ok(PathBuf::from("."));
    }
    let exe = std::env::current_exe().context("Failed to get the executable path")?;
    let dir = exe.parent().context("Executable has no parent directory")?;
    Ok(dir.to_path_buf())
}

pub async fn mkapp(state: AppState, pool: &sqlx::SqlitePool) -> Result<Router, anyhow::Error> {
    let assets_dir = release_dir()?.join("assets/dist");
    // FIXME customize 404
    let auth_layer = init_session(pool, &state.db)
        .await
        .context("Failed to initialize session store")?;

    let router = crate::router::init_router(state.clone())
        .with_state(state)
        .layer(auth_layer)
        .nest_service(ASSETS_URL_BASE, ServeDir::new(assets_dir))
        // TODO: Propagate error message in AppEnv::Dev
        .layer(CatchPanicLayer::new());
    Ok(router)
}

pub async fn init_state(
    conf: &AppConfig,
    start: bool,
) -> Result<(AppState, sqlx::SqlitePool), anyhow::Error> {
    let release_dir = release_dir()?;
    let assets = AssetManifest::load(
        ASSETS_URL_BASE.to_string(),
        &release_dir.join("assets/dist/manifest.json"),
    )
    .await
    .context("Failed to load asset manifest")?;
    let template_engine = Arc::new(init_templates(assets, &templates_dir(&release_dir)));

    let (pool, db) = init_db(conf, false).await?;

    let storage = init_storage(conf).await?;

    let mut video_transcoder = init_video_transcoder(conf, &db, storage.clone()).await?;
    if start {
        video_transcoder.start().await;
    }

    let email = Arc::new(EmailClient::new(&conf.email)?);
    let comment_email_worker = CommentEmailWorker::new(
        db.clone(),
        template_engine.clone(),
        email.clone(),
        conf.server_name.clone(),
    );
    if start {
        comment_email_worker.start();
    }
    let notification_email_worker = NotificationEmailWorker::new(
        db.clone(),
        template_engine.clone(),
        email.clone(),
        conf.server_name.clone(),
    );
    if start {
        notification_email_worker.start();
    }

    let state = AppState {
        github_client_token: conf.video_transcoding.github_client_token.clone(),
        template_engine,
        db,
        storage,
        video_transcoder: Arc::new(video_transcoder),
        email,
        comment_email_worker: Arc::new(comment_email_worker),
        notification_email_worker: Arc::new(notification_email_worker),
        dev: conf.env == AppEnv::Dev,
    };
    Ok((state, pool))
}

/// Connects to the database, creating a missing database file with
/// `create_if_missing`.
/// This is only wanted for migrations, so that a misconfigured path does not
/// silently produce an empty database when serving.
pub async fn init_db(
    conf: &AppConfig,
    create_if_missing: bool,
) -> Result<(sqlx::SqlitePool, sea_orm::DatabaseConnection), anyhow::Error> {
    let db_path = &conf.database_file;
    // https://cj.rs/blog/sqlite-pragma-cheatsheet-for-performance-and-consistency/
    let opts = sqlx::sqlite::SqliteConnectOptions::new()
        .filename(db_path)
        .create_if_missing(create_if_missing)
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
) -> anyhow::Result<VideoTranscoder> {
    let backend: Arc<dyn VideoTranscodingBackend> = match conf.video_transcoding.in_process {
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
            Arc::new(backend)
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
            Arc::new(backend)
        }
    };

    let video_transcoder = VideoTranscoder::new(db.clone(), backend);
    Ok(video_transcoder)
}
