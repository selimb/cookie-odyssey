use axum::{
    extract::{rejection::FormRejection, Path, State},
    response::{IntoResponse, Response},
    Form,
};
use sea_orm::EntityTrait;
use serde::Deserialize;

use crate::{
    journal::queries::query_journal_entry_by_id,
    utils::{
        date_utils::{date_to_sqlite, time_to_sqlite},
        serde_utils::string_trim,
    },
    AppState, FormError, RouteError, RouteResult, Templ, Toast,
};
use entities::{prelude::*, *};

#[derive(Deserialize, Debug)]
pub struct JournalEntryEdit {
    #[serde(deserialize_with = "string_trim")]
    title: String,
    date: chrono::NaiveDate,
    time: chrono::NaiveTime,
    #[serde(deserialize_with = "string_trim")]
    text: String,
}

pub async fn journal_entry_edit_get(
    state: State<AppState>,
    templ: Templ,
    Path(entry_id): Path<i32>,
) -> RouteResult {
    let result = query_journal_entry_by_id(entry_id, &state.db).await?;
    let entry_full = match result {
        Ok(entry_full) => entry_full,
        Err(err) => {
            return Ok(err.render(&templ).into_response());
        }
    };

    let ctx = minijinja::Value::from_serialize(entry_full);
    let html = templ.render_ctx("journal_entry_edit.html", ctx)?;
    Ok(html.into_response())
}

pub async fn journal_entry_edit_post(
    state: State<AppState>,
    Path(entry_id): Path<i32>,
    form: Result<Form<JournalEntryEdit>, FormRejection>,
) -> Result<Response, RouteError> {
    match form {
        Err(err) => {
            let resp = FormError::from(err).render(&state)?;
            Ok(resp.into_response())
        }
        Ok(Form(JournalEntryEdit {
            title,
            date,
            time,
            text,
        })) => {
            let data = journal_entry::ActiveModel {
                id: sea_orm::ActiveValue::Set(entry_id),
                title: sea_orm::ActiveValue::Set(title),
                date: sea_orm::ActiveValue::Set(date_to_sqlite(date)),
                time: sea_orm::ActiveValue::Set(time_to_sqlite(time)),
                text: sea_orm::ActiveValue::Set(text),
                ..Default::default()
            };
            JournalEntry::update(data).exec(&state.db).await?;
            let resp = Toast::success("Saved");
            Ok(resp.into_response())
        }
    }
}
