use std::env;

use app_config::{load_env, AppConfig};
use sea_orm_migration::prelude::*;

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    // CUSTOM: Load our own .env file and parse our config so that we can set
    //   DATABASE_URL, instead of being forced to use DATABASE_URL in our config.
    load_env()?;
    let conf = AppConfig::from_env()?;
    let db_url = conf.database_url();
    env::set_var("DATABASE_URL", db_url);

    cli::run_cli(migration::Migrator).await;
    Ok(())
}
