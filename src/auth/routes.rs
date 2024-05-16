use anyhow::{anyhow, Context as _};
use axum::{
    extract::{rejection::FormRejection, Query, State},
    response::{Html, IntoResponse, Redirect, Response},
    Form,
};
use axum_login::tower_sessions::Session;
use minijinja::context;
use sea_orm::{
    sea_query::OnConflict, ActiveValue, ColumnTrait, EntityTrait, QueryFilter, QueryOrder,
};
use serde::Deserialize;

use super::sessions::{AuthBackend, AuthError, AuthSession, Credentials};
use crate::{AppState, AuthUser, FormError, Route, RouteError, RouteResult, Templ, Toast};
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
    session: Session,
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

    // FIXME confetti
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

    let resp = [("HX-Redirect", &next)].into_response();
    Ok(resp)
}

pub async fn register_get(templ: Templ) -> RouteResult {
    let html = templ.render("register.html")?;
    Ok(html.into_response())
}

#[derive(Deserialize, Clone, Debug)]
pub struct Register {
    email: String,
    first_name: String,
    last_name: String,
    password: String,
}

pub async fn register_post(
    state: State<AppState>,
    form: Result<Form<Register>, FormRejection>,
) -> RouteResult {
    let form = match form {
        Ok(form) => form.0,
        Err(err) => {
            let resp = FormError::from(err).render(&state)?;
            return Ok(resp.into_response());
        }
    };

    let email = AuthBackend::normalize_email(form.email);
    let data = user::ActiveModel {
        admin: sea_orm::ActiveValue::Set(false),
        email: sea_orm::ActiveValue::Set(email.clone()),
        password: sea_orm::ActiveValue::Set(AuthBackend::hash_password(form.password)),
        first_name: sea_orm::ActiveValue::Set(form.first_name),
        last_name: sea_orm::ActiveValue::Set(form.last_name),
        approved: sea_orm::ActiveValue::Set(false),
        first_login: sea_orm::ActiveValue::NotSet,
        id: sea_orm::ActiveValue::NotSet,
    };
    let result = User::insert(data)
        .on_conflict(
            OnConflict::column(user::Column::Email)
                .do_nothing()
                .to_owned(),
        )
        .do_nothing()
        .exec(&state.db)
        .await?;
    match result {
        sea_orm::TryInsertResult::Conflicted => {
            let user = User::find()
                .filter(user::Column::Email.eq(email))
                .one(&state.db)
                .await?;
            match user {
                Some(u) => {
                    if u.approved {
                        let resp = FormError::new("You're already registered!").render(&state);
                        return Ok(resp.into_response());
                    } else {
                        let resp = FormError::new(
                            "You're already registered, but haven't been approved yet.",
                        )
                        .render(&state);
                        return Ok(resp.into_response());
                    }
                }
                None => {
                    return Err(RouteError::Other(
                        "Expected to find user since there is a conflict".to_string(),
                    ));
                }
            }
        }
        sea_orm::TryInsertResult::Empty => {}
        sea_orm::TryInsertResult::Inserted(_) => {}
    };

    let body = r#"
    <div class="notification is-success">
        You have been been registered!
        <br />
        You will be able to login once you have been approved.
    </div>
    "#;
    let resp = (
        [("HX-Swap", "outerHTML"), ("HX-Target", "this")],
        Html(body),
    );
    Ok(resp.into_response())
}

pub async fn logout_post(mut auth_session: AuthSession) -> RouteResult {
    auth_session.logout().await.context("Failed to logout")?;

    Ok(Redirect::to("/").into_response())
}

pub async fn forgot_password_get(_state: State<AppState>) -> RouteResult {
    todo!()
}

pub async fn forgot_password_post(_state: State<AppState>) -> RouteResult {
    todo!()
}

async fn render_user_list(
    state: &AppState,
    templ: &Templ,
    partial: bool,
) -> Result<Html<String>, RouteError> {
    let users = User::find()
        .order_by_asc(user::Column::Email)
        .all(&state.db)
        .await?;

    let ctx = context! {
        users,
        href_approve => Route::UserListApprovePost.as_path(),
        href_delete => Route::UserListDeletePost.as_path()
    };
    if partial {
        templ.render_ctx_fragment("user_list.html", ctx, "frag_user_list")
    } else {
        templ.render_ctx("user_list.html", ctx)
    }
}

pub async fn user_list_get(state: State<AppState>, templ: Templ) -> RouteResult {
    let html = render_user_list(&state, &templ, false).await?;
    Ok(html.into_response())
}

#[derive(Deserialize, Debug)]
pub struct UserApprovePost {
    user_id: i32,
}

pub async fn user_approve_post(
    state: State<AppState>,
    templ: Templ,
    form: Form<UserApprovePost>,
) -> Result<Response, Toast> {
    let r: RouteResult = async {
        let data = user::ActiveModel {
            id: ActiveValue::Set(form.user_id),
            approved: ActiveValue::Set(true),
            ..Default::default()
        };
        User::update(data).exec(&state.db).await?;
        let html = render_user_list(&state, &templ, true).await?;
        let toast = Toast::success("User has been approved");
        let resp = (toast.into_headers(), html);
        Ok(resp.into_response())
    }
    .await;
    r.map_err(Toast::error)
}

#[derive(Deserialize, Debug)]
pub struct UserDeletePost {
    user_id: i32,
}

pub async fn user_delete_post(
    state: State<AppState>,
    templ: Templ,
    form: Form<UserDeletePost>,
) -> Result<Response, Toast> {
    let r: RouteResult = async {
        User::delete_by_id(form.user_id).exec(&state.db).await?;
        let html = render_user_list(&state, &templ, true).await?;
        let toast = Toast::success("User has been deleted");
        let resp = (toast.into_headers(), html);
        Ok(resp.into_response())
    }
    .await;
    r.map_err(Toast::error)
}
