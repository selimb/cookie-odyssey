use axum::extract::{FromRef, FromRequestParts};
use std::{convert::Infallible, sync::Arc};

use crate::{
    storage::FileStore, template_engine::TemplateEngine,
    video_transcoding::daemon::VideoTranscodeDaemon,
};

#[derive(Debug, Clone)]
pub struct AppState {
    pub template_engine: Arc<TemplateEngine>,
    pub db: sea_orm::DatabaseConnection,
    pub storage: Arc<FileStore>,
    pub video_transcoder: Arc<VideoTranscodeDaemon>,
    pub dev: bool,
}

// Copied from https://github.com/tokio-rs/axum/discussions/1732#discussioncomment-4878401.
// This is necessary for using AppState in custom extractors (like [`Templ`]).
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
