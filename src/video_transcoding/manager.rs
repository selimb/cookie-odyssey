use std::{path::Path, sync::Arc};

use crate::{storage::FileStore, video_transcoding::transcode::transcode_video};
use anyhow::Context;
use entities::{prelude::VideoTranscodeTask, video_transcode_task};
use sea_orm::{ActiveModelTrait, ColumnTrait, EntityTrait, QueryFilter};
use tracing::{debug, error, info};

pub struct VideoTranscoder {
    pub storage: Arc<FileStore>,
    pub db: sea_orm::DatabaseConnection,
    pub work_dir: String,
}

impl VideoTranscoder {
    pub async fn enqueue_task(
        db: &sea_orm::DatabaseConnection,
        file_id: i32,
    ) -> anyhow::Result<()> {
        let task = video_transcode_task::ActiveModel {
            file_id: sea_orm::ActiveValue::Set(file_id),
            status: sea_orm::ActiveValue::Set(entities::video_transcode_task::TaskStatus::Pending),
            detail: sea_orm::ActiveValue::Set("".to_string()),
            created_at: sea_orm::ActiveValue::Set(chrono::Utc::now()),
            ..Default::default()
        };

        task.insert(db)
            .await
            .context("Failed to insert new video transcoding task")?;

        Ok(())
    }

    /// Polls for pending tasks and processes them all.
    pub async fn process_pending(&self) -> anyhow::Result<()> {
        let pending = self.list_pending().await?;
        debug!("Found {} pending video transcoding tasks.", pending.len());
        for task in pending {
            self.process_one(task).await?;
        }

        Ok(())
    }

    async fn list_pending(&self) -> anyhow::Result<Vec<video_transcode_task::Model>> {
        // TODO: This doesn't lock the rows, but this is meant to be run from a
        // single "process".
        VideoTranscodeTask::find()
            .filter(entities::video_transcode_task::Column::Status.eq("pending"))
            .all(&self.db)
            .await
            .context("Failed to list pending tasks")
    }

    async fn process_one(&self, task: video_transcode_task::Model) -> anyhow::Result<()> {
        let db_file = entities::file::Entity::find_by_id(task.file_id)
            .one(&self.db)
            .await
            .context("Failed to query file")?
            .with_context(|| format!("File {} not found. FK violation?", task.file_id))?;

        let work_dir = Path::new(&self.work_dir);
        let input_path = work_dir.join(db_file.id.to_string());
        let output_path = work_dir.join(format!("{}.transcoded.mp4", db_file.id));

        self.storage
            .download_to_file(db_file.bucket.clone(), db_file.key.clone(), &input_path)
            .await
            .context("Failed to download blob")?;

        let result = transcode_video(&input_path, &output_path);

        let task_id = task.id;
        let mut task: entities::video_transcode_task::ActiveModel = task.into();

        if let Err(err) = result {
            error!("Transcoding failed for task {task_id}: {err:#?}");
            task.updated_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now()));
            task.status =
                sea_orm::ActiveValue::Set(entities::video_transcode_task::TaskStatus::Failed);
            task.detail = sea_orm::ActiveValue::Set(err.to_string());
            task.update(&self.db).await?;
            return Ok(());
        }

        self.storage
            .upload_file(db_file.bucket.clone(), db_file.key.clone(), &output_path)
            .await
            .with_context(|| {
                format!(
                    "Failed to upload transcoded file at {:?} for task {}",
                    output_path, task_id
                )
            })?;

        task.updated_at = sea_orm::ActiveValue::Set(Some(chrono::Utc::now()));
        task.status =
            sea_orm::ActiveValue::Set(entities::video_transcode_task::TaskStatus::Completed);
        task.detail = sea_orm::ActiveValue::Set("".to_string());
        task.update(&self.db).await?;

        info!("Completed task {}", task_id);

        Ok(())
    }
}
