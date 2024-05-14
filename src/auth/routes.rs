use anyhow::Context as _;
use axum::{
    extract::{rejection::FormRejection, Query, State},
    http::{HeaderMap, StatusCode},
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use axum_login::tower_sessions::Session;
use sea_orm::EntityTrait;
use serde::Deserialize;
use tera::Context;

use super::sessions::{AuthSession, Credentials};
use crate::{
    auth::sessions::AuthUser,
    router::Route,
    server::AppState,
    utils::route_utils::{HtmlResult, RouteError},
};
use entities::{prelude::*, *};

#[derive(Debug, Deserialize)]
pub struct NextUrl {
    next: Option<String>,
}

pub async fn login_get(
    state: State<AppState>,
    Query(NextUrl { next }): Query<NextUrl>,
) -> HtmlResult {
    let mut context = Context::new();
    context.insert("href_register", &Route::RegisterGet.as_path());
    context.insert("href_forgot_password", &Route::ForgotPasswordGet.as_path());
    context.insert("next", &next.unwrap_or("/".to_string()));
    let resp = state.render("login.html", &context)?;
    Ok(resp)
}

pub async fn login_post(
    mut auth_session: AuthSession,
    session: Session,
    state: State<AppState>,
    form: Result<Form<Credentials>, FormRejection>,
) -> Result<Response, RouteError> {
    let mut context = Context::new();
    let creds = match form {
        Ok(form) => form.0,
        Err(err) => {
            context.insert("error", &err.body_text());
            let body = state.tera.render("common/form_error.html", &context)?;
            let resp = (
                StatusCode::UNPROCESSABLE_ENTITY,
                [
                    ("HX-Reswap", &"outerHTML".to_string()),
                    ("HX-Retarget", &"#errors".to_string()),
                ],
                Html(body),
            )
                .into_response();
            return Ok(resp);
        }
    };
    let next = creds.next.clone();
    let user: AuthUser = match auth_session
        .authenticate(creds)
        .await
        .context("auth error")?
    {
        Some(user) => user,
        None => {
            context.insert("error", "Invalid credentials");
            let body = state.tera.render("common/form_error.html", &context)?;
            let resp = (
                StatusCode::UNAUTHORIZED,
                [
                    ("HX-Reswap", &"outerHTML".to_string()),
                    ("HX-Retarget", &"#errors".to_string()),
                ],
                Html(body),
            )
                .into_response();
            return Ok(resp);
        }
    };

    auth_session
        .login(&user)
        .await
        .context("Failed to log into the session")?;

    // xxx confetti
    if user.0.first_login {
        let data = user::ActiveModel {
            id: sea_orm::ActiveValue::Set(user.0.id),
            first_login: sea_orm::ActiveValue::Set(false),
            ..Default::default()
        };
        User::update(data).exec(&state.db).await?;
        // FIXME handle
        session
            .insert("first_login", true)
            .await
            .context("Failed to update the session")?;
    }

    // let resp = ([("HX-Redirect", &next)], Redirect::to(&next)).into_response();
    let resp = [("HX-Redirect", &next)].into_response();
    Ok(resp)
}

pub async fn register_get(state: State<AppState>) -> HtmlResult {
    let mut context = Context::new();
    let resp = state.render("register.html", &context)?;
    Ok(resp)
}

pub async fn register_post(state: State<AppState>) -> HtmlResult {
    todo!()
}

pub async fn forgot_password_get(state: State<AppState>) -> HtmlResult {
    todo!()
}

pub async fn forgot_password_post(state: State<AppState>) -> HtmlResult {
    todo!()
}
