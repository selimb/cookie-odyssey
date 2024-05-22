use axum::{
    extract::{rejection::FormRejection, Path, Query, State},
    response::{IntoResponse, Response},
    Form,
};

use minijinja::context;
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};

use crate::{
    journal::queries::query_journal_by_slug,
    utils::{
        date_utils::{date_to_sqlite, time_to_sqlite},
        serde_utils::string_trim,
    },
    AppState, FormError, Route, RouteError, RouteResult, Templ,
};
use entities::{prelude::*, *};

#[derive(Deserialize, Serialize, Debug)]
pub struct JournalEntryNewPath {
    pub slug: String,
}

#[derive(Deserialize, Serialize, Debug, Default)]
pub struct JournalEntryNewQuery {
    pub date: Option<chrono::NaiveDate>,
}

#[derive(Deserialize, Debug)]
pub struct JournalEntryNew {
    journal_id: i32,
    #[serde(deserialize_with = "string_trim")]
    title: String,
    date: chrono::NaiveDate,
    time: chrono::NaiveTime,
}

pub async fn journal_entry_new_get(
    state: State<AppState>,
    templ: Templ,
    Path(slug): Path<String>,
    Query(query): Query<JournalEntryNewQuery>,
) -> RouteResult {
    let journal = query_journal_by_slug(slug, &state.db).await?;
    let journal = match journal {
        Ok(journal) => journal,
        Err(err) => {
            return Ok(err.render(&templ).into_response());
        }
    };

    let ctx = context! { journal };
    let html = templ.render_ctx("journal_entry_new.html", ctx)?;
    Ok(html.into_response())
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
        Ok(Form(JournalEntryNew {
            journal_id,
            title,
            date,
            time,
        })) => {
            let data = journal_entry::ActiveModel {
                journal_id: sea_orm::ActiveValue::Set(journal_id),
                title: sea_orm::ActiveValue::Set(title),
                date: sea_orm::ActiveValue::Set(date_to_sqlite(date)),
                time: sea_orm::ActiveValue::Set(time_to_sqlite(time)),
                ..Default::default()
            };
            let entry = JournalEntry::insert(data).exec(&state.db).await?;
            let href = Route::JournalEntryEditGet {
                entry_id: Some(entry.last_insert_id),
            }
            .as_path();
            let resp = [("HX-Location", href.as_ref())];
            Ok(resp.into_response())
        }
    }
}
