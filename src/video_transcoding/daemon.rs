use anyhow::Context;
use entities::video_transcode_task;
use std::time::Duration;
use tokio::{select, sync::mpsc, task::JoinHandle, time};
use tracing::{debug, error, info};

use crate::video_transcoding::{
    backend::traits::VideoTranscodingBackend, manager::VideoTranscodingManager,
};

enum Message {
    Process(Vec<video_transcode_task::Model>),
    Shutdown,
}

#[derive(Debug)]
pub struct VideoTranscodeDaemon {
    channel: mpsc::Sender<Message>,
    worker_handle: JoinHandle<()>,
}

impl VideoTranscodeDaemon {
    pub async fn start(
        backend: Box<dyn VideoTranscodingBackend>,
        db: sea_orm::DatabaseConnection,
    ) -> Self {
        let (tx, rx) = mpsc::channel(32);

        let handle = tokio::spawn(async move {
            run(backend, db, rx).await;
        });

        Self {
            channel: tx,
            worker_handle: handle,
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

    pub async fn process(&self, tasks: Vec<video_transcode_task::Model>) -> anyhow::Result<()> {
        debug!("Asking transcode daemon to process {} tasks", tasks.len());

        self.channel
            .send(Message::Process(tasks))
            .await
            .context("Failed to send message to transcode worker")?;

        Ok(())
    }
}

async fn run(
    backend: Box<dyn VideoTranscodingBackend>,
    db: sea_orm::DatabaseConnection,
    mut rx: mpsc::Receiver<Message>,
) -> () {
    let poll_interval = Duration::from_secs(30);
    let mut ticker = time::interval(poll_interval);
    ticker.set_missed_tick_behavior(time::MissedTickBehavior::Delay);
    let backend = backend.as_ref();

    if let Err(e) = process_pending(backend, &db).await {
        error!("Transcode (startup) error: {e:#?}");
    };

    loop {
        select! {
            _ = ticker.tick(), if !backend.is_delayed() => {
                // XXX
                if let Err(e) = process_pending(backend, &db).await {
                    error!("Transcode (interval) error: {e:#?}");
                }
            }
            message = rx.recv() => {
                match message {
                    Some(message) => {
                        match message {
                            Message::Process(tasks) => {
                                process_tasks(backend, &db, tasks).await;
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

/// Polls for pending tasks and processes them all.
async fn process_pending(
    backend: &dyn VideoTranscodingBackend,
    db: &sea_orm::DatabaseConnection,
) -> anyhow::Result<()> {
    let pending = VideoTranscodingManager::list_pending(db).await?;
    debug!("Found {} pending video transcoding tasks.", pending.len());

    process_tasks(backend, db, pending).await;
    Ok(())
}

async fn process_tasks(
    backend: &dyn VideoTranscodingBackend,
    db: &sea_orm::DatabaseConnection,
    tasks: Vec<video_transcode_task::Model>,
) {
    for task in tasks {
        let task_id = task.id;
        info!("Processing task {task_id}");
        match backend.transcode(&task).await {
            Err(err) => {
                error!("Failed task {task_id}: {err:#?}");
                VideoTranscodingManager::mark_task_error(db, task.id, err.to_string())
                    .await
                    .unwrap_or_else(|err| {
                        error!("Failed to mark task {task_id} as error: {err:#?}");
                    });
            }
            Ok(_) => {
                if backend.is_delayed() {
                    info!("Delayed task {task_id}");
                    // Do nothing, leave it as pending.
                } else {
                    info!("Completed task {task_id}");
                    VideoTranscodingManager::mark_task_completed(db, task.id)
                        .await
                        .unwrap_or_else(|err| {
                            error!("Failed to mark task {task_id} as completed: {err:#?}");
                        });
                }
            }
        }
    }
}
