use std::sync::Arc;

use app_config::AppConfig;
use axum::{response::Html, Router};
use sea_orm::{Database, DatabaseConnection};
use tera::Tera;

use crate::template_engine::init_templates;

#[derive(Clone)]
pub struct AppState {
    tera: Arc<Tera>,
    pub db: DatabaseConnection,
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

pub async fn mkapp(conf: AppConfig) -> Result<Router, String> {
    let state = init_state(conf).await?;

    let router = crate::router::router(state);
    Ok(router)
}

async fn init_state(conf: AppConfig) -> Result<AppState, String> {
    let tera = init_templates()?;
    let db = init_db(conf).await?;
    let state = AppState {
        tera: Arc::new(tera),
        db,
    };
    Ok(state)
}

async fn init_db(conf: AppConfig) -> Result<DatabaseConnection, String> {
    let db_url = conf.database_url();
    let db = Database::connect(db_url)
        .await
        .map_err(|err| format!("Failed to connect to database: {err}"));
    db
}
