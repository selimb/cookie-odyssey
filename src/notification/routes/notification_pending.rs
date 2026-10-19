use axum::response::{Html, IntoResponse as _};
use minijinja::context;
use serde::Serialize;

use crate::{
    journal::routes::JournalDayGetPath,
    notification::queries::{
        flush, flush_reach, list_pending, list_users_with_unsent_email, PendingRow,
    },
    AppState, Route, RouteError, RouteResult, Templ, Toast,
};

#[derive(Debug, Serialize)]
struct PendingItem {
    title: String,
    journal_name: String,
    datetime: chrono::NaiveDateTime,
    href: String,
}

impl From<PendingRow> for PendingItem {
    fn from(row: PendingRow) -> Self {
        let href = Route::JournalDayGet(Some(&JournalDayGetPath {
            slug: row.journal_slug,
            date: row.date,
        }))
        .as_path()
        .into_owned();
        Self {
            title: row.title,
            journal_name: row.journal_name,
            datetime: chrono::NaiveDateTime::new(row.date, row.time),
            href,
        }
    }
}

async fn pending_ctx(state: &AppState) -> Result<minijinja::Value, RouteError> {
    let items: Vec<PendingItem> = list_pending(&state.db)
        .await?
        .into_iter()
        .map(Into::into)
        .collect();
    let reach = flush_reach(&state.db).await?;
    let users_with_unsent_email = list_users_with_unsent_email(&state.db).await?.len();
    Ok(context! {
        items,
        reach,
        users_with_unsent_email,
        href_flush => Route::NotificationFlushPost.as_path(),
        href_retry_emails => Route::NotificationRetryEmailsPost.as_path(),
    })
}

pub async fn page_notification_pending_get(state: AppState, templ: Templ) -> RouteResult {
    let ctx = pending_ctx(&state).await?;
    let html = templ.render_ctx("notification_pending.html", ctx)?;
    Ok(html.into_response())
}

pub async fn hx_notification_flush_post(state: AppState, mut templ: Templ) -> RouteResult {
    let flushed = flush(&state.db).await?;
    if flushed == 0 {
        return Ok(Toast::danger("There is nothing to flush").into_response());
    }
    state.notification_email_worker.wake();

    let content = templ.render_ctx_fragment(
        "notification_pending.html",
        pending_ctx(&state).await?,
        Some("fragment_notification_pending"),
    )?;
    let badges = templ.render_nav_badges_oob(&state.db).await?;
    let toast = Toast::success("Flushed");
    let html = Html(format!("{}{}", content.0, badges.0));
    Ok((toast.into_headers(), html).into_response())
}

pub async fn hx_notification_retry_emails_post(state: AppState) -> RouteResult {
    state.notification_email_worker.wake();
    Ok(Toast::success("Retrying emails").into_response())
}
