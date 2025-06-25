use std::collections::HashMap;

use axum::{
    extract::State,
    response::{IntoResponse, Response},
    Json,
};

use crate::{storage::Bucket, AppState, RouteError};
use entities::{prelude::*, *};
use nanoid::nanoid;
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};

// SYNC
#[derive(Deserialize, Debug)]
pub struct MediaUploadUrlBody {
    filenames: Vec<String>,
    thumbnail_extension: String,
}

// SYNC
#[derive(Serialize, Debug)]
pub struct MediaUploadUrlResultItem {
    upload_method: String,
    upload_url_original: String,
    upload_url_thumbnail: String,
    upload_headers_original: HashMap<String, String>,
    upload_headers_thumbnail: HashMap<String, String>,
    file_id_original: i32,
    file_id_thumbnail: i32,
}

pub async fn media_upload_url_post(
    state: State<AppState>,
    Json(body): Json<MediaUploadUrlBody>,
) -> Result<Response, RouteError> {
    let ext_thumbnail = body.thumbnail_extension;

    // Note the most efficient algorithm (inserts could be batched), but good enough.
    let mut result: Vec<MediaUploadUrlResultItem> = Vec::with_capacity(body.filenames.len());
    for filename in body.filenames {
        let ext_original = match filename.rsplit_once('.') {
            Some((_, ext)) => format!(".{ext}"),
            None => "".to_string(),
        };
        // Since the files will often come from a Photos library, the filenames
        // are usually meaningless. Only the extension matters (for referential purposes).
        let basename = nanoid!();
        let filename_original = format!("{basename}{ext_original}");
        let filename_thumbnail = format!("{basename}_thumbnail{ext_thumbnail}");

        let upload_params_original = state
            .storage
            .get_upload_url(Bucket::Media, filename_original)
            .await?;
        let upload_params_thumbnail = state
            .storage
            .get_upload_url(Bucket::Media, filename_thumbnail)
            .await?;

        let file_data_original = file::ActiveModel {
            bucket: sea_orm::ActiveValue::Set(upload_params_original.bucket),
            key: sea_orm::ActiveValue::Set(upload_params_original.key),
            ..Default::default()
        };
        let file_db_original = File::insert(file_data_original).exec(&state.db).await?;
        let file_id_original = file_db_original.last_insert_id;

        let file_data_thumbnail = file::ActiveModel {
            bucket: sea_orm::ActiveValue::Set(upload_params_thumbnail.bucket),
            key: sea_orm::ActiveValue::Set(upload_params_thumbnail.key),
            ..Default::default()
        };
        let file_db_thumbnail = File::insert(file_data_thumbnail).exec(&state.db).await?;
        let file_id_thumbnail = file_db_thumbnail.last_insert_id;

        result.push(MediaUploadUrlResultItem {
            upload_method: upload_params_original.method,
            upload_url_original: upload_params_original.url,
            upload_url_thumbnail: upload_params_thumbnail.url,
            upload_headers_original: upload_params_original.headers,
            upload_headers_thumbnail: upload_params_thumbnail.headers,
            file_id_original,
            file_id_thumbnail,
        });
    }

    let resp = Json(result).into_response();
    Ok(resp)
}
