use axum::{
    extract::{rejection::FormRejection, Path, State},
    response::{Html, IntoResponse},
    Form,
};
use minijinja::context;
use sea_orm::EntityTrait;
use serde::Deserialize;

use crate::{
    journal::queries::{
        append_journal_entry_media, delete_journal_entry_media, query_journal_entry_by_id,
        query_media_for_journal_entry, reorder_journal_entry_media, MediaFull,
    },
    utils::serde_utils::string_trim,
    AppState, FormError, Route, RouteError, RouteResult, Templ, Toast,
};
use entities::{prelude::*, *};

#[derive(Deserialize, Debug)]
pub struct JournalEntryEdit {
    #[serde(deserialize_with = "string_trim")]
    title: String,
    #[serde(deserialize_with = "string_trim")]
    address: String,
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

    let href_get_upload_url = Route::MediaUploadUrlPost.as_path();
    let href_commit_upload = Route::JournalEntryMediaCommitPost.as_path();
    let href_publish = Route::JournalEntryPublishPost {
        entry_id: Some(entry_id),
    }
    .as_path();
    let href_journal_detail = Route::JournalDetailGet {
        slug: Some(&entry_full.journal.slug),
    }
    .as_path();

    let ctx = context! {
        ..context! {
            href_get_upload_url,
            href_commit_upload,
            href_publish,
            href_journal_detail,
            entry => entry_full.entry,
            journal => entry_full.journal,
        },
        ..get_media_list_ctx(entry_full.media_list, entry_id)
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
            address,
            date,
            time,
            text,
        })) => {
            let data = journal_entry::ActiveModel {
                id: sea_orm::ActiveValue::Set(entry_id),
                title: sea_orm::ActiveValue::Set(title),
                address: sea_orm::ActiveValue::Set(address),
                date: sea_orm::ActiveValue::Set(date),
                time: sea_orm::ActiveValue::Set(time),
                text: sea_orm::ActiveValue::Set(text),
                ..Default::default()
            };
            JournalEntry::update(data).exec(&state.db).await?;
            let resp = Toast::success("Saved");
            Ok(resp.into_response())
        }
    }
}

pub async fn journal_entry_publish_post(state: AppState, Path(entry_id): Path<i32>) -> RouteResult {
    let data = journal_entry::ActiveModel {
        id: sea_orm::ActiveValue::Set(entry_id),
        draft: sea_orm::ActiveValue::Set(false),
        ..Default::default()
    };
    JournalEntry::update(data).exec(&state.db).await?;

    let toast = Toast::success("Published");
    // Simply wipes the button.
    let html = Html("");
    let resp = (toast.into_headers(), html);
    Ok(resp.into_response())
}

// SYNC
#[derive(Deserialize, Debug)]
pub struct JournalEntryMediaCommitItem {
    pub media_type: journal_entry_media::MediaType,
    pub file_id_original: i32,
    pub width_original: i32,
    pub height_original: i32,
    pub file_id_thumbnail: i32,
    pub width_thumbnail: i32,
    pub height_thumbnail: i32,
}

// SYNC
#[derive(Deserialize, Debug)]
pub struct JournalEntryMediaCommitBody {
    pub entry_id: i32,
    pub items: Vec<JournalEntryMediaCommitItem>,
}

// Can't send JSON payloads with htmx.ajax, so we wrap the JSON in a form field.
#[derive(Deserialize, Debug)]
pub struct JournalEntryMediaCommitForm {
    json: String,
}

pub async fn journal_entry_media_commit_post(
    state: State<AppState>,
    templ: Templ,
    form: Form<JournalEntryMediaCommitForm>,
) -> RouteResult {
    let body: JournalEntryMediaCommitBody = match serde_json::from_str(&form.json) {
        Ok(body) => body,
        Err(err) => {
            return Ok((FormError::STATUS, err.to_string()).into_response());
        }
    };
    append_journal_entry_media(&body, &state.db, &state.video_transcoder).await?;

    let html = render_media_list(body.entry_id, &state, &templ).await?;
    Ok(html.into_response())
}

#[derive(Deserialize, Debug)]
pub struct JournalEntryMediaDelete {
    media_id: i32,
    entry_id: i32,
}

pub async fn journal_entry_media_delete(
    state: State<AppState>,
    templ: Templ,
    form: Result<Form<JournalEntryMediaDelete>, FormRejection>,
) -> RouteResult {
    let form = match form {
        Ok(form) => form,
        Err(err) => {
            let resp = Toast::error(err);
            return Ok(resp.into_response());
        }
    };
    delete_journal_entry_media(form.media_id, &state.db).await?;

    let toast = Toast::success("Deleted");
    let html = render_media_list(form.entry_id, &state, &templ).await?;
    let resp = (toast.into_headers(), html);
    Ok(resp.into_response())
}

#[derive(Deserialize, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Up,
    Down,
}

#[derive(Deserialize, Debug)]
pub struct JournalEntryMediaReorder {
    pub media_id: i32,
    pub entry_id: i32,
    pub order: i32,
    pub direction: Direction,
}

pub async fn journal_entry_media_reorder(
    state: State<AppState>,
    templ: Templ,
    form: Result<Form<JournalEntryMediaReorder>, FormRejection>,
) -> RouteResult {
    let form = match form {
        Ok(form) => form,
        Err(err) => {
            let resp = Toast::error(err);
            return Ok(resp.into_response());
        }
    };
    reorder_journal_entry_media(&form, &state.db).await?;

    let html = render_media_list(form.entry_id, &state, &templ).await?;
    Ok(html.into_response())
}

fn get_media_list_ctx(media_list: Vec<MediaFull>, entry_id: i32) -> minijinja::Value {
    let href_caption_edit = Route::JournalEntryMediaEditCaptionPost.as_path();
    let href_delete = Route::JournalEntryMediaDelete.as_path();
    let href_reorder = Route::JournalEntryMediaReorder.as_path();

    let ctx = context! {
        media_list,
        entry_id,
        href_caption_edit,
        href_delete,
        href_reorder,
    };
    ctx
}

async fn render_media_list(
    entry_id: i32,
    state: &AppState,
    templ: &Templ,
) -> Result<Html<String>, RouteError> {
    let media_list = query_media_for_journal_entry(entry_id, &state.db, &state.storage).await?;
    let ctx = get_media_list_ctx(media_list, entry_id);
    let html =
        templ.render_ctx_fragment("journal_entry_edit.html", ctx, Some("fragment_media_list"))?;
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
