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
pub struct JournalEntryNew {}

pub async fn journal_entry_new_get(templ: Templ) -> RouteResult {
    todo!()
}

pub async fn journal_entry_new_post(
    state: State<AppState>,
    form: Result<Form<JournalEntryNew>, FormRejection>,
) -> Result<Response, RouteError> {
    match form {
        Err(err) => {
            let resp = FormError::from(err).render(&state)?;
            Ok(resp.into_response())
        }
        Ok(form) => {
            // let data = journal::ActiveModel {
            //     name: sea_orm::ActiveValue::Set(form.name.clone()),
            //     slug: sea_orm::ActiveValue::Set(form.slug.clone()),
            //     start_date: sea_orm::ActiveValue::Set(date_to_sqlite(form.start_date)),
            //     end_date: sea_orm::ActiveValue::Set(form.end_date.map(date_to_sqlite)),
            //     ..Default::default()
            // };
            // Journal::insert(data).exec(&state.db).await?;
            // let href = Route::JournalListGet.as_path();
            // let resp = [("HX-Location", href.as_ref())];
            // Ok(resp.into_response())
            todo!()
        }
    }
}
