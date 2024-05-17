use anyhow::Context as _;
use axum::response::{IntoResponse, Redirect};

use super::super::sessions::AuthSession;
use crate::RouteResult;

pub async fn logout_post(mut auth_session: AuthSession) -> RouteResult {
    auth_session.logout().await.context("Failed to logout")?;

    Ok(Redirect::to("/").into_response())
}
