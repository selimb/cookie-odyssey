use axum::{
    extract::{Query, State},
    response::{IntoResponse, Response},
    Json,
};

use crate::{
    journal::routes::JournalEntryMediaCommitParams, storage::store::Bucket, AppState, Route,
    RouteError,
};
use entities::{prelude::*, *};
use nanoid::nanoid;
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};

// SYNC
#[derive(Deserialize, Debug)]
pub struct MediaUploadUrlQuery {
    filename: String,
    entry_id: i32,
}

// SYNC
#[derive(Serialize, Debug)]
pub struct MediaUploadUrlResult {
    upload_url: String,
    upload_method: String,
    commit_url: String,
    commit_method: String,
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
    let file_id = file_db.last_insert_id;

    let commit_url = Route::JournalEntryMediaCommitPost(Some(&JournalEntryMediaCommitParams {
        file_id,
        entry_id: query.entry_id,
    }))
    .as_path();

    let resp_body = MediaUploadUrlResult {
        upload_method: upload_params.method,
        upload_url: upload_params.url,
        commit_url: commit_url.to_string(),
        commit_method: "POST".to_string(),
    };
    let resp = Json(resp_body).into_response();
    Ok(resp)
}
