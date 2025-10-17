#[async_trait::async_trait]
pub trait VideoTranscodingBackend: Send + Sync + 'static {
    /// Returns whether [`transcode`] is delayed (e.g., queued in an external system).
    fn is_delayed(&self) -> bool;

    async fn transcode(&self, task: &entities::video_transcode_task::Model) -> anyhow::Result<()>;
}
