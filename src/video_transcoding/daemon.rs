use anyhow::Context;
use std::time::Duration;
use tokio::{select, sync::mpsc, task::JoinHandle, time};
use tracing::{debug, error};

use crate::video_transcoding::manager::VideoTranscoder;

enum Message {
    Process,
    Shutdown,
}

#[derive(Debug)]
pub struct VideoTranscodeDaemon {
    db: sea_orm::DatabaseConnection,
    channel: mpsc::Sender<Message>,
    worker_handle: JoinHandle<()>,
}

impl VideoTranscodeDaemon {
    pub async fn start(db: sea_orm::DatabaseConnection, manager: VideoTranscoder) -> Self {
        let (tx, rx) = mpsc::channel(32);

        let handle = tokio::spawn(async move {
            VideoTranscodeDaemon::run(manager, rx).await;
        });

        Self {
            db,
            channel: tx,
            worker_handle: handle,
        }
    }

    async fn run(manager: VideoTranscoder, mut rx: mpsc::Receiver<Message>) -> () {
        let poll_interval = Duration::from_secs(30);
        let mut ticker = time::interval(poll_interval);
        ticker.set_missed_tick_behavior(time::MissedTickBehavior::Delay);

        loop {
            select! {
                _ = ticker.tick() => {
                    if let Err(e) = manager.process_pending().await {
                        error!("Transcode (interval) error: {e:#?}");
                    }
                }
                message = rx.recv() => {
                    match message {
                        Some(message) => {
                            match message {
                                Message::Process => {
                                    if let Err(e) = manager.process_pending().await {
                                        error!("Transcode (message) error: {e:#?}");
                                    }
                                }
                                Message::Shutdown => {
                                    break;
                                }
                            }
                        },
                        None => {
                            break;
                        }
                    }
                }
            }
        }
    }

    pub async fn shutdown(self) -> anyhow::Result<()> {
        // TODO: Is this the cleanest way?
        self.channel
            .send(Message::Shutdown)
            .await
            .context("Failed to send shutdown message")?;
        self.worker_handle.await.context("Failed to join worker")?;
        Ok(())
    }

    pub async fn enqueue_task(&self, file_id: i32) -> anyhow::Result<()> {
        VideoTranscoder::enqueue_task(&self.db, file_id)
            .await
            .context("Failed to enqueue task")?;

        Ok(())
    }

    pub async fn notify(&self) -> anyhow::Result<()> {
        debug!("Notifying transcode worker");

        self.channel
            .send(Message::Process)
            .await
            .context("Failed to send message to transcode worker")?;

        Ok(())
    }
}
