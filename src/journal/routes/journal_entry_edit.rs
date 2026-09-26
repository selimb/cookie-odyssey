use axum::{
    extract::{rejection::FormRejection, Path, State},
    response::{Html, IntoResponse},
    Form,
};
use minijinja::context;
use sea_orm::{EntityTrait, TransactionTrait};
use serde::Deserialize;

use crate::{
    journal::queries::{
        enqueue_video_transcoding, query_journal_entry_by_id, query_media_for_journal_entry,
        sync_journal_entry_media, MediaEditorItem,
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
    // A JSON-serialized array of `MediaEditorItem`, carried in a hidden form
    // field by the client-side editor.
    media_items: String,
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

    let media_full = query_media_for_journal_entry(entry_id, &state.db, &state.storage).await?;
    // The editor is client-owned, so it is seeded with the existing Media as a
    // JSON array rather than server-rendered markup.
    let initial_items: Vec<MediaEditorItem> = media_full
        .into_iter()
        .map(|m| MediaEditorItem {
            media_type: m.media_type,
            caption: m.caption,
            file_id_original: m.file_id_original,
            width_original: m.width_original,
            height_original: m.height_original,
            file_id_thumbnail: m.file_id_thumbnail,
            width_thumbnail: m.width_thumbnail,
            height_thumbnail: m.height_thumbnail,
            url_thumbnail: m.url_thumbnail,
        })
        .collect();
    let initial_items = serde_json::to_string(&initial_items).map_err(anyhow::Error::from)?;

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
            initial_items,
        },
        ..media_editor_ctx()
    };
    let html = templ.render_ctx("journal_entry_edit.html", ctx)?;
    Ok(html.into_response())
}

// The `href_*` URLs the client-side Media editor needs. Shared between the
// edit page (this module) and the new-entry page. The editor stages everything
// in memory and submits with the form, so the only endpoint it calls directly
// is the upload-URL minter.
pub fn media_editor_ctx() -> minijinja::Value {
    let href_upload_url = Route::MediaUploadUrlPost.as_path();
    context! {
        href_upload_url,
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
            media_items,
        })) => {
            let items: Vec<MediaEditorItem> = match serde_json::from_str(&media_items) {
                Ok(items) => items,
                Err(err) => {
                    let resp = Toast::error(err);
                    return Ok(resp.into_response());
                }
            };

            // Prose and Media are saved together: update the Entry's fields and
            // reconcile its Media list in one transaction.
            let tx = state.db.begin().await?;
            let data = journal_entry::ActiveModel {
                id: sea_orm::ActiveValue::Set(entry_id),
                title: sea_orm::ActiveValue::Set(title),
                address: sea_orm::ActiveValue::Set(address),
                date: sea_orm::ActiveValue::Set(date),
                time: sea_orm::ActiveValue::Set(time),
                text: sea_orm::ActiveValue::Set(text),
                ..Default::default()
            };
            JournalEntry::update(data).exec(&tx).await?;
            let new_video_file_ids = sync_journal_entry_media(entry_id, &items, &tx).await?;
            tx.commit().await?;

            enqueue_video_transcoding(new_video_file_ids, &state.db, &state.video_transcoder)
                .await?;

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
