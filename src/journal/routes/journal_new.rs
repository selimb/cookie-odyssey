use axum::{
    extract::{rejection::FormRejection, State},
    response::{IntoResponse, Response},
    Form,
};

use sea_orm::EntityTrait;
use serde::Deserialize;

use crate::{
    utils::{date_utils::date_to_sqlite, serde_utils::string_trim},
    AppState, FormError, Route, RouteError, RouteResult, Templ,
};
use entities::{prelude::*, *};

#[derive(Deserialize, Debug)]
pub struct JournalNew {
    #[serde(deserialize_with = "string_trim")]
    pub name: String,
    #[serde(deserialize_with = "string_trim")]
    pub slug: String,
    pub start_date: chrono::NaiveDate,
    pub end_date: Option<chrono::NaiveDate>,
}

pub async fn journal_new_get(templ: Templ) -> RouteResult {
    let html = templ.render("journal_new.html")?;
    Ok(html.into_response())
}

pub async fn journal_new_post(
    state: State<AppState>,
    form: Result<Form<JournalNew>, FormRejection>,
) -> Result<Response, RouteError> {
    match form {
        Err(err) => {
            let resp = FormError::from(err).render(&state)?;
            Ok(resp.into_response())
        }
        Ok(form) => {
            let data = journal::ActiveModel {
                name: sea_orm::ActiveValue::Set(form.name.clone()),
                slug: sea_orm::ActiveValue::Set(form.slug.clone()),
                start_date: sea_orm::ActiveValue::Set(date_to_sqlite(form.start_date)),
                end_date: sea_orm::ActiveValue::Set(form.end_date.map(date_to_sqlite)),
                ..Default::default()
            };
            // TODO: Handle conflict (can't use on_conflict, since we have two unique constraints).
            Journal::insert(data).exec(&state.db).await?;
            let href = Route::JournalListGet.as_path();
            let resp = [("HX-Location", href.as_ref())];
            Ok(resp.into_response())
        }
    }
}
