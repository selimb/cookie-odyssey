use anyhow::{anyhow, Context as _};
use axum::{
    extract::{rejection::FormRejection, Query, State},
    response::IntoResponse,
    Form,
};
use minijinja::context;
use sea_orm::EntityTrait;
use serde::Deserialize;

use super::super::sessions::{AuthError, AuthSession, Credentials};
use crate::{AppState, AuthUser, FormError, Route, RouteResult, Templ};
use entities::{prelude::*, *};

#[derive(Debug, Deserialize)]
pub struct NextUrl {
    next: Option<String>,
}

pub async fn login_get(templ: Templ, Query(NextUrl { next }): Query<NextUrl>) -> RouteResult {
    let ctx = context! {
        href_register => &Route::RegisterGet.as_path(),
        href_forgot_password => &Route::ForgotPasswordGet.as_path(),
        next => &next.unwrap_or("/".to_string()),
    };
    let html = templ.render_ctx("login.html", ctx)?;
    Ok(html.into_response())
}

pub async fn login_post(
    mut auth_session: AuthSession,
    state: State<AppState>,
    form: Result<Form<Credentials>, FormRejection>,
) -> RouteResult {
    let creds = match form {
        Ok(form) => form.0,
        Err(err) => {
            let resp = FormError::from(err).render(&state)?;
            return Ok(resp.into_response());
        }
    };
    let next = creds.next.clone();
    let user: AuthUser = match auth_session.authenticate(creds).await {
        Ok(Some(user)) => user,
        Ok(None) => {
            let resp = FormError::new("Invalid credentials").render(&state)?;
            return Ok(resp.into_response());
        }
        Err(axum_login::Error::Backend(AuthError::PendingApproval)) => {
            let resp =
                FormError::new("Calm down, your approval is still pending").render(&state)?;
            return Ok(resp.into_response());
        }
        Err(err) => {
            return Err(anyhow!(err).context("Failed to authenticate").into());
        }
    };

    auth_session
        .login(&user)
        .await
        .context("Failed to log into the session")?;

    let mut trigger = "";
    if user.0.first_login {
        let data = user::ActiveModel {
            id: sea_orm::ActiveValue::Set(user.0.id),
            first_login: sea_orm::ActiveValue::Set(false),
            ..Default::default()
        };
        User::update(data).exec(&state.db).await?;
        // [confetti-evt]
        trigger = "app.confetti";
    }

    let resp = [("HX-Location", next.as_ref()), ("HX-Trigger", trigger)].into_response();
    Ok(resp)
}
