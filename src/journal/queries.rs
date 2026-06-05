use std::collections::HashMap;

use sea_orm::{
    ActiveModelTrait, ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait,
    PaginatorTrait, QueryFilter, QueryOrder, Statement, TransactionTrait,
};

use entities::{prelude::*, *};
use serde::{Deserialize, Serialize};

use crate::{
    storage::FileStore,
    video_transcoding::{daemon::VideoTranscoder, manager::VideoTranscodingManager},
    NotFound, RouteError,
};

use super::routes::{Direction, MediaReorderBody};

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
    pub url_original: String,
    pub width_original: i32,
    pub height_original: i32,
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
// It covers both committed items (with a persisted `JournalEntryMedia` `id` and
// a signed `url_thumbnail`, sent to the edit page) and items that only exist
// client-side until the Entry is created (new-entry page), where `id` is null
// and `url_thumbnail` is unused.
// SYNC MediaEditorItem
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct MediaEditorItem {
    pub id: Option<i32>,
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

// Builds the initial Media list for the client-side editor.
// Only the Thumbnail is signed -- the editor renders Thumbnails, never the
// full-size Media.
pub async fn query_media_editor_items(
    entry_id: i32,
    db: &DatabaseConnection,
    storage: &FileStore,
) -> Result<Vec<MediaEditorItem>, RouteError> {
    let medias_db = JournalEntryMedia::find()
        .filter(journal_entry_media::Column::JournalEntryId.eq(entry_id))
        .order_by_asc(journal_entry_media::Column::Order)
        .all(db)
        .await?;

    let thumbnail_ids: Vec<i32> = medias_db.iter().map(|m| m.thumbnail_file_id).collect();
    let files_db = File::find()
        .filter(file::Column::Id.is_in(thumbnail_ids))
        .all(db)
        .await?;
    let mut files_db_by_id: HashMap<i32, file::Model> = HashMap::new();
    for file in files_db {
        files_db_by_id.insert(file.id, file);
    }

    let mut items: Vec<MediaEditorItem> = Vec::with_capacity(medias_db.len());
    for media in medias_db {
        let file_thumbnail = files_db_by_id
            .remove(&media.thumbnail_file_id)
            .expect("Should be non-null");
        let url_thumbnail = storage
            .sign_url(file_thumbnail.bucket, file_thumbnail.key)
            .await?;

        items.push(MediaEditorItem {
            id: Some(media.id),
            media_type: media.media_type,
            caption: media.caption,
            file_id_original: media.file_id,
            width_original: media.width,
            height_original: media.height,
            file_id_thumbnail: media.thumbnail_file_id,
            width_thumbnail: media.thumbnail_width,
            height_thumbnail: media.thumbnail_height,
            url_thumbnail,
        });
    }
    Ok(items)
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
            url_original,
            width_original: media.width,
            height_original: media.height,
            url_thumbnail,
            width_thumbnail: media.thumbnail_width,
            height_thumbnail: media.thumbnail_height,
        };
        media_list.push(m);
    }
    Ok(media_list)
}

// Appends the given Media items to an Entry, in order, after any existing
// Media. Returns the new `JournalEntryMedia` ids in the same order as `items`.
//
// Generic over the connection so it can run inside a transaction (used by the
// new-entry create flow, which creates the Entry and its Media atomically).
pub async fn insert_journal_entry_media<C: ConnectionTrait>(
    entry_id: i32,
    items: &[MediaEditorItem],
    conn: &C,
) -> Result<Vec<i32>, DbErr> {
    let next_order = JournalEntryMedia::find()
        .filter(journal_entry_media::Column::JournalEntryId.eq(entry_id))
        .count(conn)
        .await? as i32;

    let mut ids = Vec::with_capacity(items.len());
    for (index, item) in items.iter().enumerate() {
        let model = journal_entry_media::ActiveModel {
            journal_entry_id: sea_orm::ActiveValue::Set(entry_id),
            media_type: sea_orm::ActiveValue::Set(item.media_type),
            order: sea_orm::ActiveValue::Set(next_order + index as i32),
            file_id: sea_orm::ActiveValue::Set(item.file_id_original),
            width: sea_orm::ActiveValue::Set(item.width_original),
            height: sea_orm::ActiveValue::Set(item.height_original),
            thumbnail_file_id: sea_orm::ActiveValue::Set(item.file_id_thumbnail),
            thumbnail_width: sea_orm::ActiveValue::Set(item.width_thumbnail),
            thumbnail_height: sea_orm::ActiveValue::Set(item.height_thumbnail),
            caption: sea_orm::ActiveValue::Set(item.caption.trim().to_string()),
            id: sea_orm::ActiveValue::NotSet, // Auto-incremented.
        }
        .insert(conn)
        .await?;
        ids.push(model.id);
    }

    Ok(ids)
}

// Enqueues video transcoding for any Video items, then kicks off processing.
pub async fn enqueue_video_transcoding(
    items: &[MediaEditorItem],
    db: &DatabaseConnection,
    video_transcoder: &VideoTranscoder,
) -> anyhow::Result<()> {
    let mut tasks = Vec::new();
    for item in items {
        if item.media_type == journal_entry_media::MediaType::Video {
            let task = VideoTranscodingManager::enqueue_task(db, item.file_id_original).await?;
            tasks.push(task);
        }
    }
    if !tasks.is_empty() {
        video_transcoder.process(tasks).await?;
    }
    Ok(())
}

pub async fn delete_journal_entry_media(
    // Don't like referencing upper layers here, but this is easier.
    media_id: i32,
    db: &DatabaseConnection,
) -> Result<(), DbErr> {
    let media = JournalEntryMedia::find_by_id(media_id).one(db).await?;

    let media = match media {
        None => {
            return Ok(());
        }
        Some(media) => media,
    };
    let order = media.order;

    let tx = db.begin().await?;

    JournalEntryMedia::delete_by_id(media_id).exec(&tx).await?;

    let q = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Sqlite,
        r#"
        UPDATE journal_entry_media
        SET "order" = "order" - 1
        WHERE "journal_entry_id" = ? AND "order" >= ?
        "#,
        [media.journal_entry_id.into(), order.into()],
    );
    tx.execute(q).await?;

    tx.commit().await?;

    Ok(())
}

pub async fn reorder_journal_entry_media(
    // Don't like referencing upper layers here, but this is easier.
    params: &MediaReorderBody,
    db: &DatabaseConnection,
) -> Result<(), DbErr> {
    let order_src = params.order;
    let order_dst = match params.direction {
        Direction::Up => order_src - 1,
        Direction::Down => order_src + 1,
    };

    let q = Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Sqlite,
        // "SELECT $1, $2, $3",
        r#"
        UPDATE journal_entry_media
        SET "order" = (CASE WHEN "order" = $1 THEN $2 ELSE $1 END)
        WHERE "journal_entry_id" = $3 AND ("order" = $1 OR "order" = $2)
        "#,
        [order_src.into(), order_dst.into(), params.entry_id.into()],
    );
    db.execute(q).await?;

    Ok(())
}
