use std::sync::Arc;

use app_config::AppConfig;
use axum::{
    http::StatusCode,
    response::{Html, IntoResponse},
    routing::get,
    Router,
};
use sea_orm::{Database, DatabaseConnection};
use tera::Tera;

use crate::routes;

#[derive(Clone)]
pub struct AppState {
    tera: Arc<Tera>,
    // This is
    db: DatabaseConnection,
}

impl AppState {
    pub fn render(
        &self,
        template_name: &str,
        context: &tera::Context,
    ) -> Result<Html<String>, TemplateError> {
        match self.tera.render(template_name, context) {
            Ok(s) => Ok(Html(s)),
            Err(err) => Err(TemplateError(err)),
        }
    }
}

pub struct TemplateError(tera::Error);

// Inspired by https://github.com/Altair-Bueno/axum-template/blob/main/src/engine/tera.rs
impl IntoResponse for TemplateError {
    fn into_response(self) -> axum::response::Response {
        (StatusCode::INTERNAL_SERVER_ERROR, self.0.to_string()).into_response()
    }
}

pub async fn mkapp(conf: AppConfig) -> Result<Router, String> {
    let state = init_state(conf).await?;

    let router = Router::new()
        .route("/", get(routes::home))
        .with_state(state);
    Ok(router)
}

async fn init_state(conf: AppConfig) -> Result<AppState, String> {
    let tera = init_tera()?;
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

fn init_tera() -> Result<Tera, String> {
    Tera::new("templates/**/*.html").map_err(|err| format!("Failed to initialize tera: {err}"))
}
