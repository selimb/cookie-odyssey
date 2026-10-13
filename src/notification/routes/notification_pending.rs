use axum::response::{Html, IntoResponse as _};
use minijinja::context;
use serde::Serialize;

use crate::{
    journal::routes::JournalDayGetPath,
    notification::queries::{
        count_pending, count_unread, flush, flush_reach, list_pending, list_users_with_unemailed,
    },
    AppState, AuthSession, Route, RouteError, RouteResult, Templ, Toast,
};

#[derive(Debug, Serialize)]
struct PendingItem {
    title: String,
    journal_name: String,
    datetime: chrono::NaiveDateTime,
    href: String,
}

async fn pending_ctx(state: &AppState) -> Result<minijinja::Value, RouteError> {
    let items: Vec<PendingItem> = list_pending(&state.db)
        .await?
        .into_iter()
        .map(|row| PendingItem {
            href: Route::JournalDayGet(Some(&JournalDayGetPath {
                slug: row.journal_slug,
                date: row.date,
            }))
            .as_path()
            .into_owned(),
            title: row.title,
            journal_name: row.journal_name,
            datetime: chrono::NaiveDateTime::new(row.date, row.time),
        })
        .collect();
    let reach = flush_reach(&state.db).await?;
    let unemailed_users = list_users_with_unemailed(&state.db).await?.len();
    Ok(context! {
        items,
        reach,
        unemailed_users,
        email_configured => state.email.is_configured(),
        href_flush => Route::NotificationFlushPost.as_path(),
        href_retry_emails => Route::NotificationRetryEmailsPost.as_path(),
    })
}

pub async fn page_notification_pending_get(state: AppState, templ: Templ) -> RouteResult {
    let ctx = pending_ctx(&state).await?;
    let html = templ.render_ctx("notification_pending.html", ctx)?;
    Ok(html.into_response())
}

pub async fn hx_notification_flush_post(
    state: AppState,
    templ: Templ,
    session: AuthSession,
) -> RouteResult {
    let flushed = flush(&state.db).await?;
    if flushed == 0 {
        return Ok(Toast::danger("There is nothing to flush").into_response());
    }
    state.notification_email_worker.wake();

    let user_id = session.user.as_ref().expect("Should be authenticated").0.id;
    let ctx = context! {
        ..context! {
            // The default values were computed before the Flush.
            unread_count => count_unread(&state.db, user_id).await?,
            pending_count => count_pending(&state.db).await?,
        },
        ..pending_ctx(&state).await?
    };
    let content = templ.render_ctx_fragment(
        "notification_pending.html",
        ctx.clone(),
        Some("fragment_notification_pending"),
    )?;
    let badges = templ.render_ctx("common/nav_badges_oob.html", ctx)?;
    let toast = Toast::success("Flushed");
    let html = Html(format!("{}{}", content.0, badges.0));
    Ok((toast.into_headers(), html).into_response())
}

pub async fn hx_notification_retry_emails_post(state: AppState) -> RouteResult {
    state.notification_email_worker.wake();
    Ok(Toast::success("Retrying emails").into_response())
}
