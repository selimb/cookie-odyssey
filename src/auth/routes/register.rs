use axum::{
    extract::{rejection::FormRejection, State},
    response::{Html, IntoResponse},
    Form,
};

use sea_orm::{sea_query::OnConflict, ColumnTrait, EntityTrait, QueryFilter};
use serde::Deserialize;

use super::super::sessions::AuthBackend;
use crate::{AppState, FormError, RouteError, RouteResult, Templ};
use entities::{prelude::*, *};

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
    <div class="alert alert-success">
        You have been registered!
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
