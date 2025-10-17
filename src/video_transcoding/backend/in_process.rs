use std::{path::Path, sync::Arc};

use anyhow::Context;
use sea_orm::EntityTrait;

use crate::{
    storage::FileStore,
    video_transcoding::{
        backend::traits::VideoTranscodingBackend, manager::VideoTranscodingManager,
        transcode::transcode_video,
    },
};

#[derive(Debug)]
pub struct InProcessVideoTranscoder {
    pub storage: Arc<FileStore>,
    pub db: sea_orm::DatabaseConnection,
    pub work_dir: String,
}

#[async_trait::async_trait]
impl VideoTranscodingBackend for InProcessVideoTranscoder {
    fn is_delayed(&self) -> bool {
        false
    }

    async fn transcode(&self, task: &entities::video_transcode_task::Model) -> anyhow::Result<()> {
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

        transcode_video(&input_path, &output_path).await?;

        let output_key = VideoTranscodingManager::get_output_storage_key(&db_file.key);

        self.storage
            .upload_file(db_file.bucket.clone(), output_key.clone(), &output_path)
            .await
            .with_context(|| {
                format!(
                    "Failed to upload transcoded file at {:?} for task {}",
                    output_path, task.id
                )
            })?;

        VideoTranscodingManager::update_file_key(&self.db, &self.storage, db_file.id, output_key)
            .await
            .context("Failed to update file key")?;

        Ok(())
    }
}
