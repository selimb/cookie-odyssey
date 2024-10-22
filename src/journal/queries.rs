use sea_orm::{
    ColumnTrait, ConnectionTrait, DatabaseConnection, DbErr, EntityTrait, PaginatorTrait,
    QueryFilter, QueryOrder, Statement, TransactionTrait,
};

use entities::{prelude::*, *};
use serde::Serialize;

use crate::{storage::FileStore, NotFound, RouteError};

use super::routes::{Direction, JournalEntryMediaCommitParams, JournalEntryMediaReorder};

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
    pub file_id: i32,
    pub url: String,
    pub caption: String,
}

#[derive(Debug)]
pub struct JournalEntryFull {
    pub entry: journal_entry::Model,
    pub journal: journal::Model,
    pub media_list: Vec<MediaFull>,
}

pub async fn query_journal_entry_by_id(
    id: i32,
    db: &DatabaseConnection,
    storage: &FileStore,
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
    let media_list = query_media_for_journal_entry(entry.id, db, storage).await?;

    Ok(Ok(JournalEntryFull {
        entry,
        journal,
        media_list,
    }))
}

pub async fn query_media_for_journal_entry(
    entry_id: i32,
    db: &DatabaseConnection,
    storage: &FileStore,
) -> Result<Vec<MediaFull>, RouteError> {
    let media_db = JournalEntryMedia::find()
        .filter(journal_entry_media::Column::JournalEntryId.eq(entry_id))
        .find_also_related(File)
        .order_by_asc(journal_entry_media::Column::Order)
        .all(db)
        .await?;
    let mut media_list: Vec<MediaFull> = Vec::new();
    for (media, file) in media_db {
        let file = file.expect("Should be non-null");
        let url = storage.sign_url(file.bucket, file.key).await?;
        let m = MediaFull {
            id: media.id,
            order: media.order,
            file_id: media.file_id,
            caption: media.caption,
            url,
        };
        media_list.push(m);
    }
    Ok(media_list)
}

pub async fn append_journal_entry_media(
    // Don't like referencing upper layers here, but this is easier.
    params: &JournalEntryMediaCommitParams,
    db: &DatabaseConnection,
) -> Result<(), DbErr> {
    let next_order = JournalEntryMedia::find()
        .filter(journal_entry_media::Column::JournalEntryId.eq(params.entry_id))
        .count(db)
        .await?;

    let data = journal_entry_media::ActiveModel {
        file_id: sea_orm::ActiveValue::Set(params.file_id),
        journal_entry_id: sea_orm::ActiveValue::Set(params.entry_id),
        order: sea_orm::ActiveValue::Set(next_order as i32),
        ..Default::default()
    };
    JournalEntryMedia::insert(data).exec(db).await?;

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
    params: &JournalEntryMediaReorder,
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
