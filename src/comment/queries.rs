use itertools::{zip_eq, Itertools};
use sea_orm::{
    ColumnTrait, DatabaseConnection, DbErr, EntityTrait, LoaderTrait, QueryFilter, QueryOrder,
};
use serde::{Deserialize, Serialize};

/// What a Comment is on.
///
/// - Journal: neither `date` nor `getaway_id` is set.
/// - Day: only `date` is set.
/// - Getaway: only `getaway_id` is set.
///
/// The database does not allow both, see [comment-scope-check].
#[derive(Clone, Deserialize, Serialize, Debug)]
pub struct CommentScope {
    pub journal_id: i32,
    pub date: Option<chrono::NaiveDate>,
    pub getaway_id: Option<i32>,
}

pub struct AddCommentToJournal<'a> {
    pub scope: &'a CommentScope,
    pub user_id: i32,
    pub text: String,
}

pub async fn add_comment_to_journal(
    params: AddCommentToJournal<'_>,
    db: &DatabaseConnection,
) -> Result<(), DbErr> {
    let created_at = chrono::Utc::now().timestamp();
    let data = entities::journal_comment::ActiveModel {
        created_at: sea_orm::ActiveValue::Set(created_at),
        date: sea_orm::ActiveValue::Set(params.scope.date),
        getaway_id: sea_orm::ActiveValue::Set(params.scope.getaway_id),
        journal_id: sea_orm::ActiveValue::Set(params.scope.journal_id),
        user_id: sea_orm::ActiveValue::Set(params.user_id),
        text: sea_orm::ActiveValue::Set(params.text),
        id: sea_orm::ActiveValue::NotSet,
    };

    entities::journal_comment::Entity::insert(data)
        .exec(db)
        .await?;
    Ok(())
}

#[derive(Serialize, Debug)]
pub struct JournalCommentUser {
    id: i32,
    first_name: String,
    last_name: String,
}

#[derive(Serialize, Debug)]
pub struct JournalComment {
    id: i32,
    text: String,
    created_at: chrono::DateTime<chrono::Utc>,
    created_by: Option<JournalCommentUser>,
}

pub async fn query_comments_for_journal(
    scope: &CommentScope,
    db: &DatabaseConnection,
) -> Result<Vec<JournalComment>, DbErr> {
    let mut q = entities::journal_comment::Entity::find()
        .filter(entities::journal_comment::Column::JournalId.eq(scope.journal_id))
        .order_by_asc(entities::journal_comment::Column::CreatedAt);
    q = match scope.date {
        Some(date) => q.filter(entities::journal_comment::Column::Date.eq(date)),
        None => q.filter(entities::journal_comment::Column::Date.is_null()),
    };
    q = match scope.getaway_id {
        Some(getaway_id) => q.filter(entities::journal_comment::Column::GetawayId.eq(getaway_id)),
        None => q.filter(entities::journal_comment::Column::GetawayId.is_null()),
    };
    let comments = q.all(db).await?;
    let users = comments.load_one(entities::user::Entity, db).await?;

    let ret = zip_eq(comments, users)
        .map(|(comment, user)| JournalComment {
            id: comment.id,
            text: comment.text,
            created_at: chrono::DateTime::from_timestamp(comment.created_at, 0)
                .expect("Should be a valid timestamp"),
            created_by: user.map(|user| JournalCommentUser {
                id: user.id,
                first_name: user.first_name,
                last_name: user.last_name,
            }),
        })
        .collect_vec();

    Ok(ret)
}
