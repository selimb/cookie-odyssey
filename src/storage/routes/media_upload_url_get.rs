use axum::{
    extract::{Query, State},
    response::{IntoResponse, Response},
    Json,
};

use crate::{storage::store::Bucket, AppState, RouteError};
use entities::{prelude::*, *};
use nanoid::nanoid;
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Debug)]
pub struct MediaUploadUrlQuery {
    filename: String,
}

#[derive(Serialize, Debug)]
pub struct MediaUploadUrlResult {
    // TODO Not very secure, but meh.
    file_id: i32,
    url: String,
    method: String,
}

pub async fn media_upload_url_get(
    state: State<AppState>,
    query: Query<MediaUploadUrlQuery>,
) -> Result<Response, RouteError> {
    let ext = match query.filename.rsplit_once('.') {
        Some((_, ext)) => format!(".{ext}"),
        None => "".to_string(),
    };
    let basename = nanoid!();
    let filename = format!("{basename}{ext}");

    let upload_params = state
        .storage
        .get_upload_url(Bucket::Media, filename)
        .await?;

    let file_data = file::ActiveModel {
        bucket: sea_orm::ActiveValue::Set(upload_params.bucket),
        key: sea_orm::ActiveValue::Set(upload_params.key),
        ..Default::default()
    };
    let file_db = File::insert(file_data).exec(&state.db).await?;

    let resp_body = MediaUploadUrlResult {
        file_id: file_db.last_insert_id,
        method: upload_params.method,
        url: upload_params.url,
    };
    let resp = Json(resp_body).into_response();
    Ok(resp)
}
