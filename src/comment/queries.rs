use sea_orm::{
    ColumnTrait, DatabaseConnection, DbErr, EntityTrait, LoaderTrait, QueryFilter, QueryOrder,
};

use crate::utils::date_utils::date_to_sqlite;

pub struct AddCommentToJournal {
    pub journal_id: i32,
    pub user_id: i32,
    pub text: String,
    pub date: Option<chrono::NaiveDate>,
}

pub async fn add_comment_to_journal(
    params: AddCommentToJournal,
    db: &DatabaseConnection,
) -> Result<(), DbErr> {
    let created_at = chrono::Utc::now().timestamp();
    let data = entities::journal_comment::ActiveModel {
        created_at: sea_orm::ActiveValue::Set(created_at),
        date: sea_orm::ActiveValue::Set(params.date.map(date_to_sqlite)),
        journal_id: sea_orm::ActiveValue::Set(params.journal_id),
        user_id: sea_orm::ActiveValue::Set(params.user_id),
        text: sea_orm::ActiveValue::Set(params.text),
        id: sea_orm::ActiveValue::NotSet,
    };

    entities::journal_comment::Entity::insert(data)
        .exec(db)
        .await?;
    Ok(())
}

pub struct JournalCommentUser {
    id: i32,
    first_name: String,
    last_name: String,
}

pub struct JournalComment {
    id: i32,
    text: i32,
    created_at: chrono::Utc,
    created_by: JournalCommentUser,
}

pub async fn query_comments_for_journal(
    journal_id: i32,
    date: Option<chrono::NaiveDate>,
    db: &DatabaseConnection,
) -> Result<Vec<JournalComment>, DbErr> {
    let mut q = entities::journal_comment::Entity::find()
        .filter(entities::journal_comment::Column::JournalId.eq(journal_id))
        .order_by_asc(entities::journal_comment::Column::CreatedAt);
    if let Some(date) = date {
        q = q.filter(entities::journal_comment::Column::Date.eq(date_to_sqlite(date)))
    }
    let comments = q.all(db).await?;
    let users = comments.load_one(entities::user::Entity, db).await?;

    println!("users: {:#?}", users);

    Ok(vec![])
}
