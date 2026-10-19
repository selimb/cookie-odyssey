//! Background worker that emails Recipients about their unread Notifications.
use std::sync::Arc;

use anyhow::Context;
use minijinja::context;
use sea_orm::EntityTrait;
use serde::Serialize;
use tokio::sync::Notify;
use tracing::{error, info};

use crate::{
    email::{EmailClient, EmailMessage, EmailRecipient},
    journal::routes::JournalDayGetPath,
    notification::queries::{
        list_unsent_email_for_user, list_users_with_unsent_email, mark_email_sent, NotificationRow,
    },
    template_engine::TemplateEngine,
    Route,
};
use entities::{prelude::*, user};

#[derive(Debug)]
pub struct NotificationEmailWorker {
    notify: Arc<Notify>,
    state: Arc<WorkerState>,
}

#[derive(Debug)]
struct WorkerState {
    db: sea_orm::DatabaseConnection,
    templates: Arc<TemplateEngine>,
    email: Arc<EmailClient>,
    server_name: String,
}

impl NotificationEmailWorker {
    pub fn new(
        db: sea_orm::DatabaseConnection,
        templates: Arc<TemplateEngine>,
        email: Arc<EmailClient>,
        server_name: String,
    ) -> Self {
        Self {
            notify: Arc::new(Notify::new()),
            state: Arc::new(WorkerState {
                db,
                templates,
                email,
                server_name,
            }),
        }
    }

    /// Spawns the worker.
    /// There must be at most one, so that emails are never sent twice.
    pub fn start(&self) {
        let notify = self.notify.clone();
        let state = self.state.clone();
        tokio::spawn(async move {
            loop {
                // NOTE: A wake-up that arrives during a pass leaves a permit
                //   behind, which causes another pass. That pass finds
                //   nothing left to do.
                notify.notified().await;
                if let Err(err) = run_pass(&state).await {
                    error!("Notification email pass failed: {err:#?}");
                }
            }
        });
    }

    pub fn wake(&self) {
        self.notify.notify_one();
    }
}

async fn run_pass(state: &WorkerState) -> anyhow::Result<()> {
    let user_ids = list_users_with_unsent_email(&state.db)
        .await
        .context("Failed to list users with unsent email")?;
    info!("Emailing notifications to {} users", user_ids.len());
    for user_id in user_ids {
        if let Err(err) = email_user(state, user_id).await {
            // Leave the rows unsent so that they can be retried in the next pass.
            error!("Failed to email notifications to user {user_id}: {err:#?}");
        }
    }
    Ok(())
}

async fn email_user(state: &WorkerState, user_id: i32) -> anyhow::Result<()> {
    let Some(user) = User::find_by_id(user_id)
        .one(&state.db)
        .await
        .context("Failed to fetch user")?
    else {
        return Ok(());
    };
    let rows = list_unsent_email_for_user(&state.db, user_id)
        .await
        .context("Failed to list unsent notifications")?;
    if rows.is_empty() {
        return Ok(());
    }
    let ids: Vec<i32> = rows.iter().map(|r| r.id).collect();
    let msg = render_email(state, &user, rows)?;
    state
        .email
        .send(msg)
        .await
        .context("Failed to send email")?;
    mark_email_sent(&state.db, ids)
        .await
        .context("Email was sent, but failed to mark it as sent")?;
    Ok(())
}

#[derive(Serialize)]
struct EmailEntry {
    title: String,
    date: String,
    href: String,
}

#[derive(Serialize)]
struct EmailGroup {
    journal_name: String,
    entries: Vec<EmailEntry>,
}

/// Groups rows by Journal, in order of first appearance.
/// `rows` are expected to be in the order of `list_unsent_email_for_user`.
fn group_rows(rows: Vec<NotificationRow>, server_name: &str) -> Vec<EmailGroup> {
    // Use a `Vec` instead of a `HashMap` to preserve the order of first appearance.
    let mut groups: Vec<(String, EmailGroup)> = Vec::new();
    for row in rows {
        let href = Route::JournalDayGet(Some(&JournalDayGetPath {
            slug: row.journal_slug.clone(),
            date: row.date,
        }))
        .as_url(server_name);
        let entry = EmailEntry {
            title: row.title,
            date: row.date.format("%a, %b %-d, %Y").to_string(),
            href,
        };
        match groups
            .iter_mut()
            .find(|(slug, _)| *slug == row.journal_slug)
        {
            Some((_, group)) => group.entries.push(entry),
            None => groups.push((
                row.journal_slug,
                EmailGroup {
                    journal_name: row.journal_name,
                    entries: vec![entry],
                },
            )),
        }
    }
    groups.into_iter().map(|(_, g)| g).collect()
}

fn subject_for(groups: &[EmailGroup]) -> String {
    let first = &groups[0].journal_name;
    match groups.len() - 1 {
        0 => format!("New in {first}"),
        1 => format!("New in {first} and 1 other Journal"),
        n => format!("New in {first} and {n} other Journals"),
    }
}

fn render_email(
    state: &WorkerState,
    user: &user::Model,
    rows: Vec<NotificationRow>,
) -> anyhow::Result<EmailMessage> {
    let groups = group_rows(rows, &state.server_name);
    let subject = subject_for(&groups);
    let ctx = context! {
        first_name => &user.first_name,
        groups,
        href_profile => Route::ProfileGet.as_url(&state.server_name),
    };
    let html = state
        .templates
        .get_template("email/notifications.html")?
        .render(&ctx)?;
    let text = state
        .templates
        .get_template("email/notifications.txt")?
        .render(&ctx)?;
    Ok(EmailMessage {
        to: EmailRecipient {
            name: format!("{} {}", user.first_name, user.last_name),
            address: user.email.clone(),
        },
        subject,
        text,
        html,
    })
}
