//! Background worker that emails Recipients about their unread Notifications.
//! Flush and Retry only wake it up; they never send emails themselves.
use std::sync::Arc;

use anyhow::Context as _;
use minijinja::context;
use serde::Serialize;
use tokio::sync::Notify;
use tracing::{error, info, warn};

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
use sea_orm::EntityTrait as _;

#[derive(Debug)]
pub struct NotificationEmailWorker {
    notify: Arc<Notify>,
    deps: Arc<Deps>,
}

#[derive(Debug)]
struct Deps {
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
            deps: Arc::new(Deps {
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
        let deps = self.deps.clone();
        tokio::spawn(async move {
            loop {
                // NOTE: A wake-up that arrives during a pass leaves a permit
                //   behind, which causes another pass. That pass finds
                //   nothing left to do.
                notify.notified().await;
                if let Err(err) = run_pass(&deps).await {
                    error!("Notification email pass failed: {err:#?}");
                }
            }
        });
    }

    pub fn wake(&self) {
        self.notify.notify_one();
    }
}

async fn run_pass(deps: &Deps) -> anyhow::Result<()> {
    if !deps.email.is_configured() {
        warn!("Email is not configured, leaving notification emails unsent");
        return Ok(());
    }
    let user_ids = list_users_with_unsent_email(&deps.db).await?;
    info!("Emailing notifications to {} users", user_ids.len());
    for user_id in user_ids {
        if let Err(err) = email_user(deps, user_id).await {
            // Leave the rows unsent so that they roll over to the next pass.
            error!("Failed to email notifications to user {user_id}: {err:#?}");
        }
    }
    Ok(())
}

async fn email_user(deps: &Deps, user_id: i32) -> anyhow::Result<()> {
    let Some(user) = User::find_by_id(user_id).one(&deps.db).await? else {
        return Ok(());
    };
    let rows = list_unsent_email_for_user(&deps.db, user_id).await?;
    if rows.is_empty() {
        return Ok(());
    }
    let ids: Vec<i32> = rows.iter().map(|r| r.id).collect();
    let msg = render_email(deps, &user, rows)?;
    deps.email.send(msg).await?;
    mark_email_sent(&deps.db, ids)
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
/// Rows are expected to be in trip order already.
fn group_rows(rows: Vec<NotificationRow>, server_name: &str) -> Vec<EmailGroup> {
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
    deps: &Deps,
    user: &user::Model,
    rows: Vec<NotificationRow>,
) -> anyhow::Result<EmailMessage> {
    let groups = group_rows(rows, &deps.server_name);
    let subject = subject_for(&groups);
    let ctx = context! {
        first_name => &user.first_name,
        groups,
        href_profile => Route::ProfileGet.as_url(&deps.server_name),
    };
    let html = deps
        .templates
        .get_template("email/notifications.html")?
        .render(&ctx)?;
    let text = deps
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
