use axum::{
    extract::{rejection::FormRejection, Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
    Form,
};

use chrono::Datelike;
use minijinja::context;
use sea_orm::{EntityTrait, TransactionTrait};
use serde::{Deserialize, Serialize};

use crate::{
    journal::{
        queries::{
            enqueue_video_transcoding, query_journal_by_slug, sync_journal_entry_media,
            MediaEditorItem,
        },
        routes::media_editor_ctx,
    },
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
}

#[derive(Deserialize, Debug)]
pub struct JournalEntryNew {
    journal_id: i32,
    #[serde(deserialize_with = "string_trim")]
    title: String,
    date: chrono::NaiveDate,
    time: chrono::NaiveTime,
    // A JSON-serialized array of `MediaEditorItem`, carried in a hidden form
    // field by the client-side editor. Empty when no Media was added.
    media_items: String,
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

    let ctx = context! {
        ..context! {
            journal,
            default_date,
        },
        ..media_editor_ctx()
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
            date,
            time,
            media_items,
        })) => {
            let items: Vec<MediaEditorItem> = match serde_json::from_str(&media_items) {
                Ok(items) => items,
                Err(err) => {
                    return Ok((StatusCode::BAD_REQUEST, err.to_string()).into_response());
                }
            };

            // Create the Entry and its Media atomically: backing out before
            // Create leaves nothing, and Create should never half-succeed.
            let tx = state.db.begin().await?;
            let data = journal_entry::ActiveModel {
                journal_id: sea_orm::ActiveValue::Set(journal_id),
                title: sea_orm::ActiveValue::Set(title),
                date: sea_orm::ActiveValue::Set(date),
                time: sea_orm::ActiveValue::Set(time),
                ..Default::default()
            };
            let entry = JournalEntry::insert(data).exec(&tx).await?;
            let entry_id = entry.last_insert_id;
            let new_video_file_ids = sync_journal_entry_media(entry_id, &items, &tx).await?;
            tx.commit().await?;

            // Video transcoding is a background side-effect, kicked off after
            // the Entry and its Media are durably committed.
            enqueue_video_transcoding(new_video_file_ids, &state.db, &state.video_transcoder)
                .await?;

            let href = Route::JournalEntryEditGet {
                entry_id: Some(entry_id),
            }
            .as_path();
            let resp = [("HX-Location", href.as_ref())];
            Ok(resp.into_response())
        }
    }
}
