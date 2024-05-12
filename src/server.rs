use std::sync::Arc;

use anyhow::Context;
use app_config::AppConfig;
use axum::{response::Html, Router};
use sea_orm::{Database, DatabaseConnection};
use tera::Tera;

use crate::{
    storage::{init_storage, FileStore},
    template_engine::init_templates,
};

#[derive(Clone)]
pub struct AppState {
    pub tera: Arc<Tera>,
    pub db: DatabaseConnection,
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
    let state = init_state(conf).await?;

    let router = crate::router::init_router(conf).with_state(state);
    Ok(router)
}

async fn init_state(conf: &AppConfig) -> Result<AppState, anyhow::Error> {
    let tera = init_templates()?;
    let db = init_db(conf).await?;
    let storage = init_storage(conf).context("Failed to initialize file storage")?;
    let state = AppState {
        tera: Arc::new(tera),
        db,
        storage,
    };
    Ok(state)
}

async fn init_db(conf: &AppConfig) -> Result<DatabaseConnection, anyhow::Error> {
    let db_url = conf.database_url();
    let db = Database::connect(db_url)
        .await
        .context("Failed to connect to database");
    db
}
