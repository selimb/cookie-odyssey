use std::collections::HashMap;

use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter, QueryOrder,
};

use entities::{prelude::*, *};
use serde::{Deserialize, Serialize};

use crate::{
    storage::FileStore,
    video_transcoding::{daemon::VideoTranscoder, manager::VideoTranscodingManager},
    NotFound, RouteError,
};

pub async fn query_journal_by_slug(
    slug: String,
    db: &DatabaseConnection,
) -> Result<Result<journal::Model, NotFound>, DbErr> {
    let journal = Journal::find()
        .filter(journal::Column::Slug.eq(slug))
        .one(db)
        .await?;
    match journal {
        Some(journal) => Ok(Ok(journal)),
        None => Ok(Err(NotFound::for_entity("journal"))),
    }
}

#[derive(Serialize, Debug)]
pub struct MediaFull {
    pub id: i32,
    pub order: i32,
    pub caption: String,
    pub media_type: journal_entry_media::MediaType,
    pub file_id_original: i32,
    pub url_original: String,
    pub width_original: i32,
    pub height_original: i32,
    pub file_id_thumbnail: i32,
    pub url_thumbnail: String,
    pub width_thumbnail: i32,
    pub height_thumbnail: i32,
}

#[derive(Debug)]
pub struct JournalEntryFull {
    pub entry: journal_entry::Model,
    pub journal: journal::Model,
}

pub async fn query_journal_entry_by_id(
    id: i32,
    db: &DatabaseConnection,
) -> Result<Result<JournalEntryFull, NotFound>, anyhow::Error> {
    let entry = JournalEntry::find_by_id(id)
        .find_also_related(Journal)
        .one(db)
        .await?;
    let (entry, journal) = match entry {
        Some((entry, Some(journal))) => (entry, journal),
        _ => {
            return Ok(Err(NotFound::for_entity("entry")));
        }
    };

    Ok(Ok(JournalEntryFull { entry, journal }))
}

// The shape of a single Media item as exchanged with the client-side editor.
// The editor stages its whole list in memory and submits it with the form; the
// server reconciles it against existing rows keyed by `file_id_original` (unique
// per upload), so the item carries no `JournalEntryMedia` id. `url_thumbnail` is
// the signed Thumbnail URL the editor renders; it is only meaningful server ->
// client (the edit page's initial list) and is ignored on the way back.
// SYNC MediaEditorItem
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MediaEditorItem {
    pub media_type: journal_entry_media::MediaType,
    #[serde(default)]
    pub caption: String,
    pub file_id_original: i32,
    pub width_original: i32,
    pub height_original: i32,
    pub file_id_thumbnail: i32,
    pub width_thumbnail: i32,
    pub height_thumbnail: i32,
    #[serde(default)]
    pub url_thumbnail: String,
}

pub async fn query_media_for_journal_entry(
    entry_id: i32,
    db: &DatabaseConnection,
    storage: &FileStore,
) -> Result<Vec<MediaFull>, RouteError> {
    // Collect media entries.
    let medias_db = JournalEntryMedia::find()
        .filter(journal_entry_media::Column::JournalEntryId.eq(entry_id))
        .order_by_asc(journal_entry_media::Column::Order)
        .all(db)
        .await?;

    // Collect files.
    let mut file_ids: Vec<i32> = Vec::with_capacity(medias_db.len() * 2);
    for media in &medias_db {
        file_ids.push(media.file_id);
        file_ids.push(media.thumbnail_file_id);
    }
    let files_db = File::find()
        .filter(file::Column::Id.is_in(file_ids))
        .all(db)
        .await?;
    let mut files_db_by_id: HashMap<i32, file::Model> = HashMap::new();
    for file in files_db {
        files_db_by_id.insert(file.id, file);
    }

    let mut media_list: Vec<MediaFull> = Vec::new();
    for media in medias_db {
        // Use .remove to take ownership and avoid copying.
        // Assumes that two `media` don't have the same file_id or thumbnail_file_id,
        // which should always be true.
        let file_original = files_db_by_id
            .remove(&media.file_id)
            .expect("Should be non-null");
        let file_thumbnail = files_db_by_id
            .remove(&media.thumbnail_file_id)
            .expect("Should be non-null");

        let url_original = storage
            .sign_url(file_original.bucket, file_original.key)
            .await?;
        let url_thumbnail = storage
            .sign_url(file_thumbnail.bucket, file_thumbnail.key)
            .await?;

        let m = MediaFull {
            id: media.id,
            order: media.order,
            caption: media.caption,
            media_type: media.media_type,
            file_id_original: media.file_id,
            url_original,
            width_original: media.width,
            height_original: media.height,
            file_id_thumbnail: media.thumbnail_file_id,
            url_thumbnail,
            width_thumbnail: media.thumbnail_width,
            height_thumbnail: media.thumbnail_height,
        };
        media_list.push(m);
    }
    Ok(media_list)
}

// Reconciles an Entry's Media against the editor's submitted list, in one pass:
// rows whose `file_id_original` is absent from `items` are deleted, surviving
// rows have their `order` and `caption` updated to match the submitted list, and
// items with a new `file_id_original` are inserted. The submitted order is the
// item's index in `items`. Returns the `file_id_original` of any newly-inserted
// Video items so the caller can enqueue transcoding for them.
//
// Keying on `file_id_original` (unique per upload) makes re-saving idempotent
// without the client tracking row ids. Generic over the connection so it can run
// inside a transaction (the new-entry create flow needs Entry + Media atomic).
pub async fn sync_journal_entry_media<C: ConnectionTrait>(
    entry_id: i32,
    items: &[MediaEditorItem],
    conn: &C,
) -> Result<Vec<i32>, DbErr> {
    let existing = JournalEntryMedia::find()
        .filter(journal_entry_media::Column::JournalEntryId.eq(entry_id))
        .all(conn)
        .await?;
    let mut existing_by_file_id: HashMap<i32, journal_entry_media::Model> =
        existing.into_iter().map(|m| (m.file_id, m)).collect();

    let submitted_file_ids: std::collections::HashSet<i32> =
        items.iter().map(|item| item.file_id_original).collect();

    // Delete rows the editor no longer lists.
    let to_delete: Vec<i32> = existing_by_file_id
        .values()
        .filter(|m| !submitted_file_ids.contains(&m.file_id))
        .map(|m| m.id)
        .collect();
    if !to_delete.is_empty() {
        JournalEntryMedia::delete_many()
            .filter(journal_entry_media::Column::Id.is_in(to_delete))
            .exec(conn)
            .await?;
    }

    let mut new_video_file_ids = Vec::new();
    for (index, item) in items.iter().enumerate() {
        let order = index as i32;
        let caption = item.caption.trim().to_string();
        match existing_by_file_id.remove(&item.file_id_original) {
            // Existing row: only order and caption can change.
            Some(model) => {
                let data = journal_entry_media::ActiveModel {
                    id: sea_orm::ActiveValue::Set(model.id),
                    order: sea_orm::ActiveValue::Set(order),
                    caption: sea_orm::ActiveValue::Set(caption),
                    ..Default::default()
                };
                JournalEntryMedia::update(data).exec(conn).await?;
            }
            // New row.
            None => {
                let data = journal_entry_media::ActiveModel {
                    journal_entry_id: sea_orm::ActiveValue::Set(entry_id),
                    media_type: sea_orm::ActiveValue::Set(item.media_type),
                    order: sea_orm::ActiveValue::Set(order),
                    file_id: sea_orm::ActiveValue::Set(item.file_id_original),
                    width: sea_orm::ActiveValue::Set(item.width_original),
                    height: sea_orm::ActiveValue::Set(item.height_original),
                    thumbnail_file_id: sea_orm::ActiveValue::Set(item.file_id_thumbnail),
                    thumbnail_width: sea_orm::ActiveValue::Set(item.width_thumbnail),
                    thumbnail_height: sea_orm::ActiveValue::Set(item.height_thumbnail),
                    caption: sea_orm::ActiveValue::Set(caption),
                    id: sea_orm::ActiveValue::NotSet, // Auto-incremented.
                };
                JournalEntryMedia::insert(data).exec(conn).await?;
                if item.media_type == journal_entry_media::MediaType::Video {
                    new_video_file_ids.push(item.file_id_original);
                }
            }
        }
    }

    Ok(new_video_file_ids)
}

// Enqueues video transcoding for the given original file ids, then kicks off
// processing.
pub async fn enqueue_video_transcoding(
    file_ids: Vec<i32>,
    db: &DatabaseConnection,
    video_transcoder: &VideoTranscoder,
) -> anyhow::Result<()> {
    if file_ids.is_empty() {
        return Ok(());
    }
    let mut tasks = Vec::with_capacity(file_ids.len());
    for file_id in file_ids {
        tasks.push(VideoTranscodingManager::enqueue_task(db, file_id).await?);
    }
    video_transcoder.process(tasks).await?;
    Ok(())
}
