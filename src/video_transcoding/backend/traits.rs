use std::fmt::Debug;

// async_trait adds a `must_use` of its own to the boxed future.
#[allow(clippy::double_must_use)]
#[async_trait::async_trait]
pub trait VideoTranscodingBackend: Send + Sync + 'static + Debug {
    /// Returns whether [`transcode`] is delayed (e.g., queued in an external system).
    fn is_delayed(&self) -> bool;

    async fn transcode(&self, task: &entities::video_transcode_task::Model) -> anyhow::Result<()>;
}
