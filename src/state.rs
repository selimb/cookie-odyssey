use axum::{
    async_trait,
    extract::{FromRef, FromRequestParts},
};
use std::{convert::Infallible, sync::Arc};

use crate::{storage::FileStore, template_engine::TemplateEngine};

#[derive(Debug, Clone)]
pub struct AppState {
    pub template_engine: Arc<TemplateEngine>,
    pub db: sea_orm::DatabaseConnection,
    pub storage: Arc<FileStore>,
    pub dev: bool,
}

// Copied from https://github.com/tokio-rs/axum/discussions/1732#discussioncomment-4878401
#[async_trait]
impl<S> FromRequestParts<S> for AppState
where
    Self: FromRef<S>, // <---- added this line
    S: Send + Sync,
{
    type Rejection = Infallible;

    async fn from_request_parts(
        _parts: &mut axum::http::request::Parts,
        state: &S,
    ) -> Result<Self, Self::Rejection> {
        Ok(Self::from_ref(state))
    }
}
