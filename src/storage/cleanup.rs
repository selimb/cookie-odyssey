use std::collections::HashSet;

use crate::storage::{store::FileKey, Bucket, FileStore};
use anyhow::Context;
use itertools::Itertools;
use sea_orm::{EntityTrait, FromQueryResult, Statement};
use tracing::info;

pub struct StorageCleanup {
    pub storage: FileStore,
    pub db: sea_orm::DatabaseConnection,
    pub dry_run: bool,
}

impl StorageCleanup {
    pub async fn run<F>(&self, confirm: F) -> Result<(), anyhow::Error>
    where
        F: FnOnce() -> bool,
    {
        let bucket = Bucket::Media;

        let storage_files = self.storage.list_files(&bucket).await?;
        let storage_files_count = storage_files.len();

        let all_db_files: Vec<ListDbFilesItem> = self.list_db_files(&bucket).await?;
        let all_db_files_count = all_db_files.len();

        //
        // Start by collecting orphaned DB files.
        //
        // Note that we might have two (or more) rows with the
        // same key but with different `orphaned` status -- that shouldn't
        // happen when using the website, but may happen when doing ad-hoc
        // thumbnail data migrations.
        let mut db_file_keys: HashSet<FileKey> = HashSet::new();
        let mut db_files_to_delete: Vec<i32> = Vec::new();
        for db_file in all_db_files {
            let file_key = db_file.key;
            if db_file.orphaned {
                db_files_to_delete.push(db_file.id);
            } else {
                db_file_keys.insert(file_key);
            }
        }

        let mut storage_files_to_delete: HashSet<String> = HashSet::new();

        for storage_file_key in storage_files.iter() {
            if !db_file_keys.contains(storage_file_key) {
                storage_files_to_delete.insert(storage_file_key.clone());
            }
        }

        if self.dry_run {
            info!(
                "Would delete {} / {} files from storage:\n{}",
                storage_files_to_delete.len(),
                storage_files_count,
                storage_files_to_delete.iter().join("\n")
            );
            info!(
                "Would delete {} / {} files from database:\n{}",
                db_files_to_delete.len(),
                all_db_files_count,
                db_files_to_delete.iter().join("\n")
            );
            return Ok(());
        }

        info!(
            "Would delete {} / {} files from storage",
            storage_files_to_delete.len(),
            storage_files_count,
        );
        info!(
            "Would delete {} / {} files from database",
            db_files_to_delete.len(),
            all_db_files_count,
        );
        let ok = confirm();
        if !ok {
            return Ok(());
        }

        for file_key in storage_files_to_delete {
            info!("Deleting storage file: {file_key}");
            self.storage.delete_file(&bucket, &file_key).await?;
        }
        for db_file_id in db_files_to_delete {
            info!("Deleting database file: {db_file_id}");
            entities::file::Entity::delete_by_id(db_file_id)
                .exec(&self.db)
                .await
                .context(format!("Failed to delete database file: {db_file_id}"))?;
        }
        Ok(())
    }

    async fn list_db_files(&self, bucket: &Bucket) -> Result<Vec<ListDbFilesItem>, sea_orm::DbErr> {
        let bucket = bucket.to_name(&self.storage.conf);
        let q = Statement::from_sql_and_values(
            sea_orm::DatabaseBackend::Sqlite,
            r#"
            SELECT
                f.id,
                f.key,
                CASE
                    WHEN m1.id IS NULL AND m2.id IS NULL THEN TRUE
                    ELSE FALSE
                END AS orphaned
            FROM
                file f
                LEFT JOIN journal_entry_media m1 ON m1.file_id = f.id
                LEFT JOIN journal_entry_media m2 ON m2.thumbnail_file_id = f.id
            WHERE f.bucket = ?
            "#,
            [bucket.into()],
        );
        let rows = ListDbFilesItem::find_by_statement(q).all(&self.db).await?;

        Ok(rows)
    }
}

#[derive(Debug, FromQueryResult)]
struct ListDbFilesItem {
    pub id: i32,
    pub key: String,
    pub orphaned: bool,
}
