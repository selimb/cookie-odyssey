use std::fmt::Debug;

// Newer clippy flags the `#[must_use]` that `async_trait` generates.
#[allow(clippy::double_must_use)]
#[async_trait::async_trait]
pub trait VideoTranscodingBackend: Send + Sync + 'static + Debug {
    /// Returns whether [`transcode`] is delayed (e.g., queued in an external system).
    fn is_delayed(&self) -> bool;

    async fn transcode(&self, task: &entities::video_transcode_task::Model) -> anyhow::Result<()>;
}
