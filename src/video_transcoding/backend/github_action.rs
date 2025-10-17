use std::sync::Arc;

use anyhow::Context;
use azure_storage::prelude::BlobSasPermissions;
use sea_orm::EntityTrait;
use time::OffsetDateTime;

use crate::{
    storage::FileStore,
    video_transcoding::{
        backend::traits::VideoTranscodingBackend, manager::VideoTranscodingManager,
        routes::VideoTranscodeCallbackQuery,
    },
    Route,
};

#[derive(Debug)]
pub struct GithubActionVideoTranscoder {
    pub storage: Arc<FileStore>,
    pub db: sea_orm::DatabaseConnection,
    pub server_name: String,
    pub github_workflow_url: String,
    pub github_token: String,
    pub reqwest: reqwest::Client,
}

impl GithubActionVideoTranscoder {
    /// https://docs.github.com/en/rest/actions/workflows?apiVersion=2022-11-28#create-a-workflow-dispatch-event
    pub async fn trigger_github_workflow(
        &self,
        input_url: &str,
        output_url: &str,
        task_id: i32,
        callback_url: &str,
    ) -> anyhow::Result<()> {
        self.reqwest
            .post(&self.github_workflow_url)
            .bearer_auth(&self.github_token)
            .header("X-GitHub-Api-Version", "2022-11-28")
            // Matches [gh-transcode-inputs]
            .json(&serde_json::json!({
                "ref": "main",
                "inputs": {
                    "input_url": input_url,
                    "output_url": output_url,
                    "task_id": task_id,
                    "callback_url": callback_url,
                }
            }))
            .send()
            .await
            .context("Failed to send request")?
            .error_for_status()?;

        Ok(())
    }
}

#[async_trait::async_trait]
impl VideoTranscodingBackend for GithubActionVideoTranscoder {
    fn is_delayed(&self) -> bool {
        true
    }

    async fn transcode(&self, task: &entities::video_transcode_task::Model) -> anyhow::Result<()> {
        let db_file = entities::file::Entity::find_by_id(task.file_id)
            .one(&self.db)
            .await
            .context("Failed to query file")?
            .with_context(|| format!("File {} not found. FK violation?", task.file_id))?;

        let bucket = db_file.bucket;
        let input_key = db_file.key;
        let output_key = VideoTranscodingManager::get_output_storage_key(&input_key);

        let expiry = OffsetDateTime::now_utc() + time::Duration::hours(1);
        let input_url = self
            .storage
            .sign_url2(
                bucket.clone(),
                input_key.clone(),
                BlobSasPermissions {
                    read: true,
                    ..Default::default()
                },
                expiry,
            )
            .await
            .context("Failed to generate input_url")?;

        let output_url = self
            .storage
            .sign_url2(
                bucket.clone(),
                output_key.clone(),
                BlobSasPermissions {
                    create: true,
                    write: true,
                    ..Default::default()
                },
                expiry,
            )
            .await
            .context("Failed to generate output_url")?;

        let callback_url = Route::VideoTranscodeCallbackPost(Some(&VideoTranscodeCallbackQuery {
            task_id: task.id,
            output_key,
        }))
        .as_url(&self.server_name);

        self.trigger_github_workflow(&input_url, &output_url, task.id, &callback_url)
            .await?;

        Ok(())
    }
}
