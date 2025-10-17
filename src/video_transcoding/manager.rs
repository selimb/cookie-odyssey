use anyhow::{anyhow, Context};
use entities::{prelude::VideoTranscodeTask, video_transcode_task};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};

use crate::storage::{FileKey, FileStore};

pub struct VideoTranscodingManager {}

/// Interface for managing video transcoding.
impl VideoTranscodingManager {
    pub async fn enqueue_task(
        db: &sea_orm::DatabaseConnection,
        file_id: i32,
    ) -> anyhow::Result<video_transcode_task::Model> {
        let data = video_transcode_task::ActiveModel {
            file_id: sea_orm::ActiveValue::Set(file_id),
            status: sea_orm::ActiveValue::Set(entities::video_transcode_task::TaskStatus::Pending),
            detail: sea_orm::ActiveValue::Set("".to_string()),
            created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
            ..Default::default()
        };

        let task = data
            .insert(db)
            .await
            .context("Failed to insert new video transcoding task")?;

        Ok(task)
    }

    pub async fn mark_task_completed(
        db: &sea_orm::DatabaseConnection,
        task_id: i32,
    ) -> anyhow::Result<()> {
        let data = video_transcode_task::ActiveModel {
            status: sea_orm::ActiveValue::Set(
                entities::video_transcode_task::TaskStatus::Completed,
            ),
            updated_at: sea_orm::ActiveValue::Set(Some(chrono::Utc::now())),
            detail: sea_orm::ActiveValue::Set("".to_string()),
            ..Default::default()
        };
        let _update_result = VideoTranscodeTask::update_many()
            .set(data)
            .filter(entities::video_transcode_task::Column::Id.eq(task_id))
            .exec(db)
            .await?;
        Ok(())
    }

    pub async fn mark_task_error(
        db: &sea_orm::DatabaseConnection,
        task_id: i32,
        detail: String,
    ) -> anyhow::Result<()> {
        let data = video_transcode_task::ActiveModel {
            status: sea_orm::ActiveValue::Set(entities::video_transcode_task::TaskStatus::Failed),
            updated_at: sea_orm::ActiveValue::Set(Some(chrono::Utc::now())),
            detail: sea_orm::ActiveValue::Set(detail),
            ..Default::default()
        };
        let _update_result = VideoTranscodeTask::update_many()
            .set(data)
            .filter(entities::video_transcode_task::Column::Id.eq(task_id))
            .exec(db)
            .await?;
        Ok(())
    }

    pub async fn get_task_by_id(
        db: &sea_orm::DatabaseConnection,
        task_id: i32,
    ) -> anyhow::Result<Option<video_transcode_task::Model>> {
        let task = VideoTranscodeTask::find_by_id(task_id)
            .one(db)
            .await
            .context("Failed to query video transcoding task")?;
        Ok(task)
    }

    pub async fn list_pending(
        db: &sea_orm::DatabaseConnection,
    ) -> anyhow::Result<Vec<video_transcode_task::Model>> {
        // TODO: This doesn't lock the rows, but this is meant to be run from a
        // single "process".
        VideoTranscodeTask::find()
            .filter(entities::video_transcode_task::Column::Status.eq("pending"))
            .all(db)
            .await
            .context("Failed to list pending tasks")
    }

    pub fn get_output_storage_key(input_key: &str) -> FileKey {
        format!("{}.transcoded.mp4", input_key)
    }

    /// Updates the `key` on the database `File`, and deletes the old file from storage.
    pub async fn update_file_key(
        db: &sea_orm::DatabaseConnection,
        storage: &FileStore,
        file_id: i32,
        output_key: FileKey,
    ) -> anyhow::Result<()> {
        let file = entities::file::Entity::find_by_id(file_id)
            .one(db)
            .await
            .context("Failed to query file")?
            .with_context(|| format!("File {file_id} not found"))?;

        if file.key == output_key {
            return Err(anyhow!("Output key is the same as input key: {}", file.key));
        }

        let update_data = entities::file::ActiveModel {
            key: sea_orm::ActiveValue::Set(output_key),
            ..Default::default()
        };

        entities::file::Entity::update_many()
            .set(update_data)
            .filter(entities::file::Column::Id.eq(file_id))
            .exec(db)
            .await
            .context("Failed to update file key")?;

        storage
            .delete_file(&file.bucket, &file.key)
            .await
            .context("Failed to delete file from storage")?;

        Ok(())
    }
}
