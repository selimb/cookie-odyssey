use axum::{
    extract::State,
    response::{Html, IntoResponse, Response},
    Form,
};
use minijinja::context;
use sea_orm::{ActiveValue, EntityTrait, QueryOrder};
use serde::Deserialize;

use crate::{AppState, Route, RouteError, RouteResult, Templ, Toast};
use entities::{prelude::*, *};

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
