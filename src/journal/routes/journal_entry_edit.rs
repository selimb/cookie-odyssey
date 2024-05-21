use axum::{
    extract::{rejection::FormRejection, Path, Query, State},
    response::{Html, IntoResponse, Response},
    Form,
};
use minijinja::context;
use sea_orm::{EntityTrait, QueryFilter};
use serde::{Deserialize, Serialize};

use crate::{
    journal::queries::{
        append_journal_entry_media, query_journal_entry_by_id, query_media_for_journal_entry,
    },
    utils::{
        date_utils::{date_to_sqlite, time_to_sqlite},
        serde_utils::string_trim,
    },
    AppState, FormError, Route, RouteError, RouteResult, Templ, Toast,
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
    let result = query_journal_entry_by_id(entry_id, &state.db, &state.storage).await?;
    let entry_full = match result {
        Ok(entry_full) => entry_full,
        Err(err) => {
            return Ok(err.render(&templ).into_response());
        }
    };

    let href_upload = Route::MediaUploadUrlGet.as_path();
    let href_caption_edit = Route::JournalEntryMediaEditCaptionPost.as_path();
    let ctx = context! {
        ..minijinja::Value::from_serialize(entry_full),
        ..context! {
            href_upload,
            href_caption_edit,
        }
    };
    let html = templ.render_ctx("journal_entry_edit.html", ctx)?;
    Ok(html.into_response())
}

pub async fn journal_entry_edit_post(
    state: State<AppState>,
    Path(entry_id): Path<i32>,
    form: Result<Form<JournalEntryEdit>, FormRejection>,
) -> RouteResult {
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

#[derive(Serialize, Deserialize, Debug)]
pub struct JournalEntryMediaCommitParams {
    pub file_id: i32,
    pub entry_id: i32,
}

pub async fn journal_entry_media_commit_post(
    state: State<AppState>,
    templ: Templ,
    Query(params): Query<JournalEntryMediaCommitParams>,
) -> RouteResult {
    append_journal_entry_media(&params, &state.db).await?;

    let html = render_media_list(params.entry_id, &state, &templ).await?;
    Ok(html.into_response())
}

pub async fn journal_entry_media_reorder(state: State<AppState>) -> RouteResult {
    todo!()
}

async fn render_media_list(
    entry_id: i32,
    state: &AppState,
    templ: &Templ,
) -> Result<Html<String>, RouteError> {
    let media_list = query_media_for_journal_entry(entry_id, &state.db, &state.storage).await?;

    let ctx = context! {media_list};
    let html = templ.render_ctx_fragment("journal_entry_edit.html", ctx, "fragment_media_list")?;

    Ok(html)
}

#[derive(Deserialize, Debug)]
pub struct JournalEntryMediaCaptionEdit {
    media_id: i32,
    #[serde(deserialize_with = "string_trim")]
    caption: String,
}

pub async fn journal_entry_media_caption_edit(
    state: State<AppState>,
    form: Result<Form<JournalEntryMediaCaptionEdit>, FormRejection>,
) -> RouteResult {
    let form = match form {
        Ok(form) => form,
        Err(err) => {
            let resp = Toast::error(err);
            return Ok(resp.into_response());
        }
    };

    let data = journal_entry_media::ActiveModel {
        id: sea_orm::ActiveValue::Set(form.media_id),
        caption: sea_orm::ActiveValue::Set(form.caption.clone()),
        ..Default::default()
    };
    JournalEntryMedia::update(data).exec(&state.db).await?;

    let resp = Toast::success("Caption saved");
    Ok(resp.into_response())
}
