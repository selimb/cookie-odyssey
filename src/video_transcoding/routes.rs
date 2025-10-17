use axum::extract::Query;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};

use crate::{video_transcoding::manager::VideoTranscodingManager, AppState, RouteResult};

#[derive(Debug, Serialize, Deserialize)]
pub struct VideoTranscodeCallbackQuery {
    pub task_id: i32,
    pub output_key: String,
}

pub async fn video_transcode_callback_post(
    state: AppState,
    Query(VideoTranscodeCallbackQuery {
        task_id,
        output_key,
    }): Query<VideoTranscodeCallbackQuery>,
) -> RouteResult {
    let task = match VideoTranscodingManager::get_task_by_id(&state.db, task_id).await? {
        Some(task) => task,
        None => {
            return Ok((StatusCode::NOT_FOUND, "Task not found").into_response());
        }
    };

    VideoTranscodingManager::update_file_key(&state.db, &state.storage, task.file_id, output_key)
        .await?;
    VideoTranscodingManager::mark_task_completed(&state.db, task_id).await?;

    Ok(StatusCode::OK.into_response())
}
