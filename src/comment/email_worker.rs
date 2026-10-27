//! Background worker that emails Admins about new Comments.
use std::sync::{Arc, Mutex};

use anyhow::Context;
use minijinja::context;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter};
use tokio::sync::mpsc;
use tracing::error;

use crate::{
    comment::queries::CommentScope,
    email::{EmailClient, EmailMessage, EmailRecipient},
    journal::routes::{JournalDayGetPath, JournalGetawayGetPath},
    template_engine::TemplateEngine,
    Route,
};
use entities::{journal, journal_getaway, prelude::*, user};

/// A Comment that was just added.
#[derive(Debug)]
pub struct CommentEmailJob {
    pub scope: CommentScope,
    pub user_id: i32,
    pub text: String,
}

#[derive(Debug)]
struct WorkerState {
    db: sea_orm::DatabaseConnection,
    templates: Arc<TemplateEngine>,
    email: Arc<EmailClient>,
    server_name: String,
}

/// [comment-email] Sends one Comment Email per Admin for each job, one job at
/// a time.
/// Jobs are only held in memory, so a Comment Email is lost if sending it
/// fails or the process stops first.
#[derive(Debug)]
pub struct CommentEmailWorker {
    tx: mpsc::UnboundedSender<CommentEmailJob>,
    rx: Mutex<Option<mpsc::UnboundedReceiver<CommentEmailJob>>>,
    state: Arc<WorkerState>,
}

impl CommentEmailWorker {
    pub fn new(
        db: sea_orm::DatabaseConnection,
        templates: Arc<TemplateEngine>,
        email: Arc<EmailClient>,
        server_name: String,
    ) -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self {
            tx,
            rx: Mutex::new(Some(rx)),
            state: Arc::new(WorkerState {
                db,
                templates,
                email,
                server_name,
            }),
        }
    }

    /// Spawns the consumer.
    /// Must be called at most once.
    pub fn start(&self) {
        let mut rx = self
            .rx
            .lock()
            .expect("Lock should not be poisoned")
            .take()
            .expect("Worker should only be started once");
        let state = self.state.clone();
        tokio::spawn(async move {
            while let Some(job) = rx.recv().await {
                if let Err(err) = email_admins(&state, job).await {
                    error!("Failed to send Comment Emails: {err:#?}");
                }
            }
        });
    }

    /// Queues a Comment Email for every Admin, without waiting for it to be
    /// sent.
    pub fn enqueue(&self, job: CommentEmailJob) {
        if self.tx.send(job).is_err() {
            error!("Comment email worker is not running");
        }
    }
}

async fn email_admins(state: &WorkerState, job: CommentEmailJob) -> anyhow::Result<()> {
    let commenter = User::find_by_id(job.user_id)
        .one(&state.db)
        .await
        .context("Failed to fetch commenter")?
        .context("Commenter not found")?;
    let journal = Journal::find_by_id(job.scope.journal_id)
        .one(&state.db)
        .await
        .context("Failed to fetch Journal")?
        .context("Journal not found")?;
    let getaway = match job.scope.getaway_id {
        Some(getaway_id) => Some(
            JournalGetaway::find_by_id(getaway_id)
                .one(&state.db)
                .await
                .context("Failed to fetch Getaway")?
                .context("Getaway not found")?,
        ),
        None => None,
    };
    let admins = User::find()
        .filter(user::Column::Admin.eq(true))
        .all(&state.db)
        .await
        .context("Failed to list Admins")?;

    let commenter_name = format!("{} {}", commenter.first_name, commenter.last_name);
    let target = format!(
        "{}{}",
        journal.name,
        target_suffix(&journal, job.scope.date, getaway.as_ref())
    );
    let subject = format!("{commenter_name} commented on {target}");
    let href = comment_url(&state.server_name, &journal, &job.scope);
    let text = state
        .templates
        .get_template("email/comment.txt")?
        .render(context! {
            commenter_name,
            target,
            text => job.text,
            href,
        })?;

    // Each Admin gets their own email, so that addresses are not shared.
    for admin in admins {
        let msg = EmailMessage {
            to: EmailRecipient {
                name: format!("{} {}", admin.first_name, admin.last_name),
                address: admin.email.clone(),
            },
            subject: subject.clone(),
            text: text.clone(),
            html: None,
        };
        if let Err(err) = state.email.send(msg).await {
            error!(
                "Failed to send Comment Email to Admin {}: {err:#?}",
                admin.id
            );
        }
    }
    Ok(())
}

/// Returns what follows the Journal name to say what the Comment is on:
/// nothing for a Journal, the Getaway name or the Day.
/// A Day is its number, or its date in a Scrapbook.
fn target_suffix(
    journal: &journal::Model,
    date: Option<chrono::NaiveDate>,
    getaway: Option<&journal_getaway::Model>,
) -> String {
    if let Some(getaway) = getaway {
        return format!(" ({})", getaway.name);
    }
    let Some(date) = date else {
        return String::new();
    };
    match crate::journal::day_number(journal, date) {
        Some(n) => format!(" (Day {n})"),
        None => format!(" ({})", date.format("%a, %b %-d, %Y")),
    }
}

fn comment_url(server_name: &str, journal: &journal::Model, scope: &CommentScope) -> String {
    let slug = journal.slug.clone();
    let route = match (scope.date, scope.getaway_id) {
        (Some(date), _) => Route::JournalDayGet(Some(&JournalDayGetPath { slug, date })),
        (None, Some(getaway_id)) => {
            Route::JournalGetawayGet(Some(&JournalGetawayGetPath { slug, getaway_id }))
        }
        (None, None) => Route::JournalDetailGet { slug: Some(&slug) },
    };
    route.as_url(server_name)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn journal(scrapbook: bool) -> journal::Model {
        journal::Model {
            id: 1,
            name: "Japan".into(),
            slug: "japan".into(),
            start_date: chrono::NaiveDate::from_ymd_opt(2026, 3, 1).unwrap(),
            end_date: None,
            cover_id: None,
            scrapbook,
        }
    }

    fn getaway() -> journal_getaway::Model {
        journal_getaway::Model {
            id: 7,
            journal_id: 1,
            name: "Kyoto".into(),
        }
    }

    fn date(d: u32) -> chrono::NaiveDate {
        chrono::NaiveDate::from_ymd_opt(2026, 3, d).unwrap()
    }

    #[test]
    fn suffix_journal() {
        assert_eq!(target_suffix(&journal(false), None, None), "");
    }

    #[test]
    fn suffix_day_is_numbered() {
        assert_eq!(
            target_suffix(&journal(false), Some(date(3)), None),
            " (Day 3)"
        );
    }

    #[test]
    fn suffix_day_in_scrapbook_is_dated() {
        assert_eq!(
            target_suffix(&journal(true), Some(date(3)), None),
            " (Tue, Mar 3, 2026)"
        );
    }

    #[test]
    fn suffix_getaway() {
        assert_eq!(
            target_suffix(&journal(true), None, Some(&getaway())),
            " (Kyoto)"
        );
    }

    #[test]
    fn url_per_scope() {
        let j = journal(false);
        let scope = |date, getaway_id| CommentScope {
            journal_id: 1,
            date,
            getaway_id,
        };
        assert_eq!(
            comment_url("https://x", &j, &scope(None, None)),
            "https://x/journal/japan"
        );
        assert_eq!(
            comment_url("https://x", &j, &scope(Some(date(3)), None)),
            "https://x/journal/japan/entry/2026-03-03"
        );
        assert_eq!(
            comment_url("https://x", &j, &scope(None, Some(7))),
            "https://x/journal/japan/getaway/7"
        );
    }
}
