use axum::{extract::Query, response::IntoResponse as _};
use minijinja::context;
use serde::{Deserialize, Serialize};

use crate::{
    journal::routes::JournalDayGetPath,
    notification::queries::{list_for_user, mark_all_read, NotificationRow, PAGE_SIZE},
    AppState, AuthSession, Route, RouteError, RouteResult, Templ,
};

#[derive(Debug, Serialize)]
struct NotificationItem {
    read: bool,
    title: String,
    journal_name: String,
    datetime: chrono::NaiveDateTime,
    href: String,
}

impl From<NotificationRow> for NotificationItem {
    fn from(row: NotificationRow) -> Self {
        let href = Route::JournalDayGet(Some(&JournalDayGetPath {
            slug: row.journal_slug,
            date: row.date,
        }))
        .as_path()
        .into_owned();
        Self {
            read: row.read_at.is_some(),
            title: row.title,
            journal_name: row.journal_name,
            datetime: chrono::NaiveDateTime::new(row.date, Default::default()),
            href,
        }
    }
}

fn user_id(session: &AuthSession) -> i32 {
    session.user.as_ref().expect("Should be authenticated").0.id
}

/// Queries a page of the User's Notifications, and builds the template context.
async fn list_ctx(
    state: &AppState,
    session: &AuthSession,
    before: Option<i32>,
) -> Result<minijinja::Value, RouteError> {
    let mut rows = list_for_user(&state.db, user_id(session), before).await?;
    let has_more = rows.len() as u64 > PAGE_SIZE;
    rows.truncate(PAGE_SIZE as usize);
    let href_more = match (has_more, rows.last()) {
        (true, Some(last)) => Some(
            Route::NotificationListMoreGet(Some(&NotificationListMoreQuery { before: last.id }))
                .as_path(),
        ),
        _ => None,
    };
    let items: Vec<NotificationItem> = rows.into_iter().map(Into::into).collect();
    Ok(context! {
        items,
        href_more,
    })
}

pub async fn page_notification_list_get(
    state: AppState,
    templ: Templ,
    session: AuthSession,
) -> RouteResult {
    let ctx = context! {
        ..list_ctx(&state, &session, None).await?,
        ..context! {
            href_mark_all_read => Route::NotificationMarkAllReadPost.as_path(),
        }
    };
    let html = templ.render_ctx("notification_list.html", ctx)?;
    Ok(html.into_response())
}

#[derive(Debug, Deserialize, Serialize)]
pub struct NotificationListMoreQuery {
    pub before: i32,
}

pub async fn hx_notification_list_more_get(
    state: AppState,
    templ: Templ,
    session: AuthSession,
    Query(query): Query<NotificationListMoreQuery>,
) -> RouteResult {
    let ctx = list_ctx(&state, &session, Some(query.before)).await?;
    let html = templ.render_ctx_fragment(
        "notification_list.html",
        ctx,
        Some("fragment_notification_list"),
    )?;
    Ok(html.into_response())
}

pub async fn hx_notification_mark_all_read_post(
    state: AppState,
    templ: Templ,
    session: AuthSession,
) -> RouteResult {
    mark_all_read(&state.db, user_id(&session)).await?;

    let ctx = list_ctx(&state, &session, None).await?;
    let list = templ.render_ctx_fragment(
        "notification_list.html",
        ctx,
        Some("fragment_notification_list"),
    )?;
    let badges = templ.render_ctx("common/nav_badges_oob.html", context! { unread_count => 0 })?;
    Ok(axum::response::Html(format!("{}{}", list.0, badges.0)).into_response())
}
