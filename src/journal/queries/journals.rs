use sea_orm::{ColumnTrait, DatabaseConnection, DbErr, EntityTrait, QueryFilter};

use entities::{prelude::*, *};

use crate::NotFound;

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
