use axum::{
    extract::{rejection::FormRejection, Path, State},
    http::StatusCode,
    response::{Html, IntoResponse},
    Form, Json,
};
use minijinja::context;
use sea_orm::EntityTrait;
use serde::{Deserialize, Serialize};

use crate::{
    journal::queries::{
        delete_journal_entry_media, enqueue_video_transcoding, insert_journal_entry_media,
        query_journal_entry_by_id, query_media_editor_items, reorder_journal_entry_media,
        MediaEditorItem,
    },
    utils::serde_utils::string_trim,
    AppState, FormError, Route, RouteResult, Templ, Toast,
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

pub async fn page_journal_entry_edit_get(
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

    let media_items = query_media_editor_items(entry_id, &state.db, &state.storage).await?;
    // The editor is client-owned, so it is seeded with the existing Media as a
    // JSON array rather than server-rendered markup.
    let initial_items = serde_json::to_string(&media_items).map_err(anyhow::Error::from)?;

    let href_edit = Route::JournalEntryEditPost {
        entry_id: Some(entry_id),
    }
    .as_path();
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
            href_edit,
            href_publish,
            href_journal_detail,
            entry => entry_full.entry,
            journal => entry_full.journal,
            entry_id,
            initial_items,
        },
        ..media_editor_ctx()
    };
    let html = templ.render_ctx("journal_entry_edit.html", ctx)?;
    Ok(html.into_response())
}

// The `href_*` URLs the client-side Media editor needs. Shared between the
// edit page (this module) and the new-entry page.
pub fn media_editor_ctx() -> minijinja::Value {
    let href_upload_url = Route::MediaUploadUrlPost.as_path();
    let href_commit = Route::JournalEntryMediaCommitPost.as_path();
    let href_caption = Route::JournalEntryMediaEditCaptionPost.as_path();
    let href_delete = Route::JournalEntryMediaDelete.as_path();
    let href_reorder = Route::JournalEntryMediaReorder.as_path();
    context! {
        href_upload_url,
        href_commit,
        href_caption,
        href_delete,
        href_reorder,
    }
}

pub async fn hx_journal_entry_edit_post(
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

pub async fn hx_journal_entry_publish_post(
    state: AppState,
    Path(entry_id): Path<i32>,
) -> RouteResult {
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

// SYNC MediaCommitBody
#[derive(Deserialize, Debug)]
pub struct MediaCommitBody {
    pub entry_id: i32,
    pub items: Vec<MediaEditorItem>,
}

// SYNC MediaCommitResult
#[derive(Serialize, Debug)]
pub struct MediaCommitResult {
    // The new `JournalEntryMedia` ids, in the same order as the submitted items.
    pub ids: Vec<i32>,
}

// Persists newly-uploaded Media against an existing Entry (edit page).
// The client owns the DOM, so this returns the new ids as JSON rather than a
// rendered fragment; the client uses them to wire up subsequent caption,
// reorder and delete calls.
pub async fn api_journal_entry_media_commit_post(
    state: State<AppState>,
    Json(body): Json<MediaCommitBody>,
) -> RouteResult {
    let ids = insert_journal_entry_media(body.entry_id, &body.items, &state.db).await?;
    enqueue_video_transcoding(&body.items, &state.db, &state.video_transcoder).await?;
    Ok(Json(MediaCommitResult { ids }).into_response())
}

// SYNC MediaDeleteBody
#[derive(Deserialize, Debug)]
pub struct MediaDeleteBody {
    pub media_id: i32,
}

pub async fn api_journal_entry_media_delete_post(
    state: State<AppState>,
    Json(body): Json<MediaDeleteBody>,
) -> RouteResult {
    delete_journal_entry_media(body.media_id, &state.db).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

// SYNC Direction
#[derive(Deserialize, PartialEq, Debug)]
#[serde(rename_all = "lowercase")]
pub enum Direction {
    Up,
    Down,
}

// SYNC MediaReorderBody
#[derive(Deserialize, Debug)]
pub struct MediaReorderBody {
    pub media_id: i32,
    pub entry_id: i32,
    pub order: i32,
    pub direction: Direction,
}

pub async fn api_journal_entry_media_reorder_post(
    state: State<AppState>,
    Json(body): Json<MediaReorderBody>,
) -> RouteResult {
    reorder_journal_entry_media(&body, &state.db).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}

// SYNC MediaCaptionBody
#[derive(Deserialize, Debug)]
pub struct MediaCaptionBody {
    pub media_id: i32,
    #[serde(deserialize_with = "string_trim")]
    pub caption: String,
}

pub async fn api_journal_entry_media_caption_post(
    state: State<AppState>,
    Json(body): Json<MediaCaptionBody>,
) -> RouteResult {
    let data = journal_entry_media::ActiveModel {
        id: sea_orm::ActiveValue::Set(body.media_id),
        caption: sea_orm::ActiveValue::Set(body.caption),
        ..Default::default()
    };
    JournalEntryMedia::update(data).exec(&state.db).await?;
    Ok(StatusCode::NO_CONTENT.into_response())
}
