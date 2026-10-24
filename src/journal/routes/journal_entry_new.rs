use axum::{
    extract::{rejection::FormRejection, Path, Query, State},
    response::{IntoResponse, Response},
    Form,
};

use chrono::Datelike;
use minijinja::context;
use sea_orm::{ColumnTrait, EntityTrait, QueryFilter, TransactionTrait as _};
use serde::{Deserialize, Serialize};

use crate::{
    journal::queries::{query_getaway_options, query_journal_by_slug, resolve_entry_getaway},
    utils::serde_utils::string_trim,
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
    /// The Getaway to prefill.
    /// There is no other default, so that Entries are loose unless the Admin
    /// says otherwise.
    pub getaway_id: Option<i32>,
}

#[derive(Deserialize, Debug)]
pub struct JournalEntryNew {
    journal_id: i32,
    #[serde(deserialize_with = "string_trim")]
    title: String,
    /// The name of the Getaway.
    /// Absent outside of a Scrapbook, see [getaway-field].
    #[serde(default, deserialize_with = "string_trim")]
    getaway: String,
    date: chrono::NaiveDate,
    time: chrono::NaiveTime,
}

pub async fn page_journal_entry_new_get(
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
    let default_date = query.date.map(format_input_date_value);
    let getaway_options = if journal.scrapbook {
        query_getaway_options(&state.db, journal.id).await?
    } else {
        Vec::new()
    };
    let default_getaway = match query.getaway_id {
        None => None,
        Some(getaway_id) => JournalGetaway::find_by_id(getaway_id)
            .filter(journal_getaway::Column::JournalId.eq(journal.id))
            .one(&state.db)
            .await?
            .map(|g| g.name),
    };

    let ctx = context! {
        journal,
        default_date,
        default_getaway,
        getaway_options,
    };
    let html = templ.render_ctx("journal_entry_new.html", ctx)?;
    Ok(html.into_response())
}

fn format_input_date_value(d: chrono::NaiveDate) -> String {
    let year = d.year();
    let month = d.month();
    let day = d.day();
    let value = format!("{year}-{month:0>2}-{day:0>2}");
    value
}

pub async fn page_journal_entry_new_post(
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
            getaway,
            date,
            time,
        })) => {
            let Some(journal) = Journal::find_by_id(journal_id).one(&state.db).await? else {
                let resp = FormError::new("This Journal no longer exists").render(&state)?;
                return Ok(resp.into_response());
            };
            // See [deferred-txn].
            let txn = state.db.begin().await?;
            let getaway_id = match resolve_entry_getaway(&txn, &journal, &getaway).await? {
                Ok(getaway_id) => getaway_id,
                Err(err) => {
                    return Ok(err.render(&state)?.into_response());
                }
            };
            let data = journal_entry::ActiveModel {
                journal_id: sea_orm::ActiveValue::Set(journal_id),
                getaway_id: sea_orm::ActiveValue::Set(getaway_id),
                title: sea_orm::ActiveValue::Set(title),
                date: sea_orm::ActiveValue::Set(date),
                time: sea_orm::ActiveValue::Set(time),
                ..Default::default()
            };
            let entry = JournalEntry::insert(data).exec(&txn).await?;
            txn.commit().await?;
            let href = Route::JournalEntryEditGet {
                entry_id: Some(entry.last_insert_id),
            }
            .as_path();
            let resp = [("HX-Location", href.as_ref())];
            Ok(resp.into_response())
        }
    }
}
