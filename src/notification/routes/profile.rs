use axum::{extract::rejection::FormRejection, response::IntoResponse as _, Form};
use minijinja::context;
use serde::Deserialize;

use crate::{
    notification::queries::set_email_notifications, AppState, AuthSession, Route, RouteResult,
    Templ, Toast,
};

pub async fn page_profile_get(templ: Templ, session: AuthSession) -> RouteResult {
    let user = session.user.as_ref().expect("Should be authenticated");
    let ctx = context! {
        email_notifications => user.0.email_notifications,
        href_email_notifications => Route::ProfileEmailNotificationsPost.as_path(),
    };
    let html = templ.render_ctx("profile.html", ctx)?;
    Ok(html.into_response())
}

#[derive(Debug, Deserialize)]
pub struct ProfileEmailNotificationsForm {
    // An unchecked checkbox is not sent at all.
    email_notifications: Option<String>,
}

pub async fn hx_profile_email_notifications_post(
    state: AppState,
    session: AuthSession,
    form: Result<Form<ProfileEmailNotificationsForm>, FormRejection>,
) -> RouteResult {
    let form = match form {
        Ok(form) => form,
        Err(err) => {
            return Ok(Toast::error(err).into_response());
        }
    };
    let user_id = session.user.as_ref().expect("Should be authenticated").0.id;
    let enabled = form.email_notifications.is_some();
    set_email_notifications(&state.db, user_id, enabled).await?;
    Ok(Toast::success("Saved").into_response())
}
