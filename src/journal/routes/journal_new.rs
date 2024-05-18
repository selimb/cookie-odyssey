use axum::{
    extract::{rejection::FormRejection, State},
    response::{IntoResponse, Response},
    Form,
};

use sea_orm::EntityTrait;
use serde::Deserialize;

use crate::{
    utils::date_utils::date_to_sqlite, AppState, FormError, Route, RouteError, RouteResult, Templ,
};
use entities::{prelude::*, *};

#[derive(Deserialize, Debug)]
pub struct JournalNew {
    pub name: String,
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
            Journal::insert(data).exec(&state.db).await?;
            let resp = [("HX-Location", Route::JournalListGet.as_path())];
            Ok(resp.into_response())
        }
    }
}
