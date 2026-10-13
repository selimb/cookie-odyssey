use chrono::Utc;
use sea_orm::{
    sea_query::{Expr, Query},
    ColumnTrait, ConnectionTrait, DbErr, EntityTrait, FromQueryResult, JoinType, PaginatorTrait,
    QueryFilter, QueryOrder, QuerySelect, RelationTrait, Select, Statement, TransactionTrait,
};
use serde::Serialize;

use entities::{
    notification::NotificationKind, notification_recipient::NotificationEmailStatus, prelude::*, *,
};

pub const PAGE_SIZE: u64 = 50;

/// Records a Pending Notification for a freshly published Entry.
pub async fn insert_entry_published(db: &impl ConnectionTrait, entry_id: i32) -> Result<(), DbErr> {
    let data = notification::ActiveModel {
        kind: sea_orm::ActiveValue::Set(NotificationKind::EntryPublished),
        journal_entry_id: sea_orm::ActiveValue::Set(entry_id),
        created_at: sea_orm::ActiveValue::Set(Utc::now()),
        flushed_at: sea_orm::ActiveValue::Set(None),
        ..Default::default()
    };
    Notification::insert(data).exec(db).await?;
    Ok(())
}

/// Publishes a Draft Entry, and records a Pending Notification if (and only
/// if) the Entry was actually still a Draft.
pub async fn publish_entry(db: &sea_orm::DatabaseConnection, entry_id: i32) -> Result<bool, DbErr> {
    let txn = db.begin().await?;
    let result = JournalEntry::update_many()
        .col_expr(journal_entry::Column::Draft, Expr::value(false))
        .filter(journal_entry::Column::Id.eq(entry_id))
        .filter(journal_entry::Column::Draft.eq(true))
        .exec(&txn)
        .await?;
    let published = result.rows_affected > 0;
    if published {
        insert_entry_published(&txn, entry_id).await?;
    }
    txn.commit().await?;
    Ok(published)
}

pub async fn count_unread(db: &impl ConnectionTrait, user_id: i32) -> Result<u64, DbErr> {
    NotificationRecipient::find()
        .filter(notification_recipient::Column::UserId.eq(user_id))
        .filter(notification_recipient::Column::ReadAt.is_null())
        .count(db)
        .await
}

pub async fn count_pending(db: &impl ConnectionTrait) -> Result<u64, DbErr> {
    Notification::find()
        .filter(notification::Column::FlushedAt.is_null())
        .count(db)
        .await
}

/// Delivers every Pending Notification to all approved Users.
/// Returns the number of Notifications that were flushed.
///
/// The recipients are inserted in reverse trip order, so that listing by
/// `notification_recipient.id DESC` shows the newest Flush first and each Flush
/// in trip order.
pub async fn flush(db: &sea_orm::DatabaseConnection) -> Result<u64, DbErr> {
    let txn = db.begin().await?;
    // NOTE: This takes the write lock, so no publish can slip in before the
    //   update below.
    txn.execute(Statement::from_string(
        txn.get_database_backend(),
        r#"
        INSERT INTO notification_recipient
            (notification_id, user_id, read_at, email_status)
        SELECT
            n.id,
            u.id,
            NULL,
            CASE WHEN u.email_notifications THEN 'pending' ELSE 'skipped' END
        FROM notification n
        JOIN journal_entry e ON e.id = n.journal_entry_id
        CROSS JOIN "user" u
        WHERE n.flushed_at IS NULL AND u.approved
        ORDER BY e.date DESC, e.time DESC, n.id DESC
        "#,
    ))
    .await?;
    let result = Notification::update_many()
        .col_expr(notification::Column::FlushedAt, Expr::value(Utc::now()))
        .filter(notification::Column::FlushedAt.is_null())
        .exec(&txn)
        .await?;
    txn.commit().await?;
    Ok(result.rows_affected)
}

#[derive(Debug, FromQueryResult)]
pub struct NotificationRow {
    /// The Recipient id.
    pub id: i32,
    pub read_at: Option<chrono::DateTime<Utc>>,
    pub title: String,
    pub date: chrono::NaiveDate,
    pub time: chrono::NaiveTime,
    pub journal_name: String,
    pub journal_slug: String,
}

fn rows_query() -> Select<NotificationRecipient> {
    NotificationRecipient::find()
        .join(
            JoinType::InnerJoin,
            notification_recipient::Relation::Notification.def(),
        )
        .join(
            JoinType::InnerJoin,
            notification::Relation::JournalEntry.def(),
        )
        .join(JoinType::InnerJoin, journal_entry::Relation::Journal.def())
        .select_only()
        .column(notification_recipient::Column::Id)
        .column(notification_recipient::Column::ReadAt)
        .column_as(journal_entry::Column::Title, "title")
        .column_as(journal_entry::Column::Date, "date")
        .column_as(journal_entry::Column::Time, "time")
        .column_as(journal::Column::Name, "journal_name")
        .column_as(journal::Column::Slug, "journal_slug")
}

/// Returns up to [`PAGE_SIZE`] + 1 rows, newest first, so that the caller can
/// tell whether there is another page.
pub async fn list_for_user(
    db: &impl ConnectionTrait,
    user_id: i32,
    before: Option<i32>,
) -> Result<Vec<NotificationRow>, DbErr> {
    let mut q = rows_query().filter(notification_recipient::Column::UserId.eq(user_id));
    if let Some(before) = before {
        q = q.filter(notification_recipient::Column::Id.lt(before));
    }
    q.order_by_desc(notification_recipient::Column::Id)
        .limit(PAGE_SIZE + 1)
        .into_model::<NotificationRow>()
        .all(db)
        .await
}

pub async fn mark_all_read(db: &impl ConnectionTrait, user_id: i32) -> Result<(), DbErr> {
    NotificationRecipient::update_many()
        .col_expr(
            notification_recipient::Column::ReadAt,
            Expr::value(Utc::now()),
        )
        .filter(notification_recipient::Column::UserId.eq(user_id))
        .filter(notification_recipient::Column::ReadAt.is_null())
        .exec(db)
        .await?;
    Ok(())
}

/// Marks as read all of the User's Notifications for Entries on the given Day.
pub async fn mark_day_read(
    db: &impl ConnectionTrait,
    user_id: i32,
    journal_id: i32,
    date: chrono::NaiveDate,
) -> Result<(), DbErr> {
    let entries_for_day = Query::select()
        .column((notification::Entity, notification::Column::Id))
        .from(notification::Entity)
        .join(
            JoinType::InnerJoin,
            journal_entry::Entity,
            Expr::col((notification::Entity, notification::Column::JournalEntryId))
                .equals((journal_entry::Entity, journal_entry::Column::Id)),
        )
        .and_where(
            Expr::col((journal_entry::Entity, journal_entry::Column::JournalId)).eq(journal_id),
        )
        .and_where(Expr::col((journal_entry::Entity, journal_entry::Column::Date)).eq(date))
        .to_owned();
    NotificationRecipient::update_many()
        .col_expr(
            notification_recipient::Column::ReadAt,
            Expr::value(Utc::now()),
        )
        .filter(notification_recipient::Column::UserId.eq(user_id))
        .filter(notification_recipient::Column::ReadAt.is_null())
        .filter(notification_recipient::Column::NotificationId.in_subquery(entries_for_day))
        .exec(db)
        .await?;
    Ok(())
}

#[derive(Debug, Serialize, FromQueryResult)]
pub struct PendingRow {
    pub id: i32,
    pub title: String,
    pub date: chrono::NaiveDate,
    pub time: chrono::NaiveTime,
    pub journal_name: String,
    pub journal_slug: String,
}

/// Lists Pending Notifications in the order that readers will see them.
pub async fn list_pending(db: &impl ConnectionTrait) -> Result<Vec<PendingRow>, DbErr> {
    Notification::find()
        .filter(notification::Column::FlushedAt.is_null())
        .join(
            JoinType::InnerJoin,
            notification::Relation::JournalEntry.def(),
        )
        .join(JoinType::InnerJoin, journal_entry::Relation::Journal.def())
        .select_only()
        .column(notification::Column::Id)
        .column_as(journal_entry::Column::Title, "title")
        .column_as(journal_entry::Column::Date, "date")
        .column_as(journal_entry::Column::Time, "time")
        .column_as(journal::Column::Name, "journal_name")
        .column_as(journal::Column::Slug, "journal_slug")
        .order_by_desc(journal_entry::Column::Date)
        .order_by_desc(journal_entry::Column::Time)
        .order_by_desc(notification::Column::Id)
        .into_model::<PendingRow>()
        .all(db)
        .await
}

#[derive(Debug, Serialize)]
pub struct FlushReach {
    pub users: u64,
    pub users_by_email: u64,
}

pub async fn flush_reach(db: &impl ConnectionTrait) -> Result<FlushReach, DbErr> {
    let approved = || User::find().filter(user::Column::Approved.eq(true));
    let users = approved().count(db).await?;
    let users_by_email = approved()
        .filter(user::Column::EmailNotifications.eq(true))
        .count(db)
        .await?;
    Ok(FlushReach {
        users,
        users_by_email,
    })
}

fn email_pending_query() -> Select<NotificationRecipient> {
    NotificationRecipient::find()
        .filter(notification_recipient::Column::EmailStatus.eq(NotificationEmailStatus::Pending))
        .filter(notification_recipient::Column::ReadAt.is_null())
}

/// Ids of Users who have unread Notifications that still need to be emailed.
pub async fn list_users_with_unemailed(db: &impl ConnectionTrait) -> Result<Vec<i32>, DbErr> {
    email_pending_query()
        .select_only()
        .column(notification_recipient::Column::UserId)
        .distinct()
        .into_tuple::<i32>()
        .all(db)
        .await
}

/// All of the User's unread Notifications that still need to be emailed, in
/// trip order.
pub async fn list_unemailed_for_user(
    db: &impl ConnectionTrait,
    user_id: i32,
) -> Result<Vec<NotificationRow>, DbErr> {
    rows_query()
        .filter(notification_recipient::Column::UserId.eq(user_id))
        .filter(notification_recipient::Column::EmailStatus.eq(NotificationEmailStatus::Pending))
        .filter(notification_recipient::Column::ReadAt.is_null())
        .order_by_asc(journal_entry::Column::Date)
        .order_by_asc(journal_entry::Column::Time)
        .order_by_asc(notification_recipient::Column::Id)
        .into_model::<NotificationRow>()
        .all(db)
        .await
}

/// Marks the given Recipient rows as emailed.
/// Only rows that are still pending are touched.
pub async fn mark_emailed(db: &impl ConnectionTrait, ids: Vec<i32>) -> Result<(), DbErr> {
    NotificationRecipient::update_many()
        .col_expr(
            notification_recipient::Column::EmailStatus,
            Expr::value(NotificationEmailStatus::Sent),
        )
        .filter(notification_recipient::Column::Id.is_in(ids))
        .filter(notification_recipient::Column::EmailStatus.eq(NotificationEmailStatus::Pending))
        .exec(db)
        .await?;
    Ok(())
}

pub async fn set_email_notifications(
    db: &sea_orm::DatabaseConnection,
    user_id: i32,
    enabled: bool,
) -> Result<(), DbErr> {
    let txn = db.begin().await?;
    User::update_many()
        .col_expr(user::Column::EmailNotifications, Expr::value(enabled))
        .filter(user::Column::Id.eq(user_id))
        .exec(&txn)
        .await?;
    if !enabled {
        // Turning email back on later must not send a backlog.
        NotificationRecipient::update_many()
            .col_expr(
                notification_recipient::Column::EmailStatus,
                Expr::value(NotificationEmailStatus::Skipped),
            )
            .filter(notification_recipient::Column::UserId.eq(user_id))
            .filter(
                notification_recipient::Column::EmailStatus.eq(NotificationEmailStatus::Pending),
            )
            .exec(&txn)
            .await?;
    }
    txn.commit().await?;
    Ok(())
}
