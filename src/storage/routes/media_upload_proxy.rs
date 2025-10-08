use axum::{
    body::Bytes,
    extract::{Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use serde::{Deserialize, Serialize};

use crate::{AppState, RouteError};

#[derive(Deserialize, Serialize)]
pub struct MediaUploadProxyParams {
    pub bucket: String,
    pub key: String,
}

pub async fn media_upload_proxy(
    state: State<AppState>,
    Query(params): Query<MediaUploadProxyParams>,
    body: Bytes,
) -> Result<Response, RouteError> {
    match state.storage.upload(params.bucket, params.key, body).await {
        Err(err) => Err(RouteError::from(err)),
        Ok(_) => Ok((StatusCode::CREATED).into_response()),
    }
}
