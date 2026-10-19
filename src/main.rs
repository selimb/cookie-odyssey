use std::{io::Write, path::Path};

use anyhow::Context;
use clap::{Parser, Subcommand};
use cookie_odyssey::{
    auth::sessions::AuthBackend,
    server::{init_db, init_state, release_dir},
    storage::StorageCleanup,
};
use migration::{Migrator, MigratorTrait};
use sea_orm::EntityTrait;
use tracing::info;

use app_config::{load_env, AppConfig};

fn init_tracing() -> Result<(), anyhow::Error> {
    let format = tracing_subscriber::fmt::format().without_time();
    let env_filter = tracing_subscriber::EnvFilter::builder()
        .with_default_directive(tracing_subscriber::filter::LevelFilter::INFO.into())
        .from_env()
        .context("Failed to parse RUST_LOG")?;

    let subscriber = tracing_subscriber::fmt::fmt()
        .event_format(format)
        .with_env_filter(env_filter)
        .finish();
    tracing::subscriber::set_global_default(subscriber)?;
    Ok(())
}

trait ProcessCommandExt {
    fn check_output(&mut self) -> Result<String, anyhow::Error>;
}

impl ProcessCommandExt for std::process::Command {
    fn check_output(&mut self) -> Result<String, anyhow::Error> {
        let ret = self.output().context("Failed to execute command")?;
        if !ret.status.success() {
            let mut cmd = vec![self.get_program()];
            cmd.extend(self.get_args());
            let stdout = String::from_utf8_lossy(&ret.stdout);
            let stderr = String::from_utf8_lossy(&ret.stderr);
            let err_msg = format!(
                "Command failed\n# cmd: {:?}\n# stdout: {}\n# stderr: {}",
                cmd, stdout, stderr
            );
            Err(anyhow::Error::msg(err_msg))
        } else {
            Ok(String::from_utf8_lossy(&ret.stdout).to_string())
        }
    }
}

#[derive(Parser, Debug)]
#[command()]
struct CliArgs {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    Conf,
    CreateAdmin,
    CleanupStorage {
        #[arg(long)]
        dry_run: bool,
    },
    Server,
    Check,
    Migrate {
        #[command(subcommand)]
        command: MigrateCommand,
    },
}

#[derive(Subcommand, Debug, Clone, Copy)]
enum MigrateCommand {
    /// Print one line per migration: `applied <name>` or `pending <name>`.
    Status,
    /// Apply all pending migrations.
    Up,
}

struct Cli {
    args: CliArgs,
    conf: AppConfig,
}

impl Cli {
    pub async fn run(&self) -> Result<(), anyhow::Error> {
        match self.args.command {
            Commands::Conf => self.print_conf(),
            Commands::CreateAdmin => self.create_admin().await,
            Commands::CleanupStorage { dry_run } => self.cleanup_storage(dry_run).await,
            Commands::Server => self.server().await,
            Commands::Check => self.check().await,
            Commands::Migrate { command } => self.migrate(command).await,
        }
    }

    fn print_conf(&self) -> Result<(), anyhow::Error> {
        std::io::stdout().write_fmt(format_args!("{:#?}\n", self.conf))?;
        Ok(())
    }

    async fn server(&self) -> Result<(), anyhow::Error> {
        let (state, pool) = init_state(&self.conf, true).await?;
        let app = cookie_odyssey::server::mkapp(state, &pool).await?;

        // FIXME Run migrations
        let port = 4444;
        let listener = tokio::net::TcpListener::bind(format!("0.0.0.0:{port}"))
            .await
            .context("Failed to bind TCP listener")?;
        info!("Starting server on http://localhost:{port}");
        axum::serve(listener, app).await?;
        // TODO: Graceful shutdown.
        // video_transcoder.shutdown().await?;
        Ok(())
    }

    async fn create_admin(&self) -> Result<(), anyhow::Error> {
        let email = std::process::Command::new("git")
            .args(["config", "user.email"])
            .check_output()?;
        let name = std::process::Command::new("git")
            .args(["config", "user.name"])
            .check_output()?;
        let (first_name, last_name) = name.trim().split_once(' ').context("Weird name")?;
        let password = "pass";

        let (_, db) = init_db(&self.conf).await?;
        let password_hash = AuthBackend::hash_password(password.to_string());
        let user_data = entities::user::ActiveModel {
            admin: sea_orm::ActiveValue::Set(true),
            email: sea_orm::ActiveValue::Set(AuthBackend::normalize_email(&email)),
            first_name: sea_orm::ActiveValue::Set(first_name.to_string()),
            last_name: sea_orm::ActiveValue::Set(last_name.to_string()),
            password: sea_orm::ActiveValue::Set(password_hash),
            approved: sea_orm::ActiveValue::Set(true),
            first_login: sea_orm::ActiveValue::NotSet,
            email_notifications: sea_orm::ActiveValue::NotSet,
            id: sea_orm::ActiveValue::NotSet,
        };
        entities::user::Entity::insert(user_data)
            .exec(&db)
            .await
            .context("Failed to insert user")?;

        info!("Created user: '{}'", email);
        Ok(())
    }

    async fn cleanup_storage(&self, dry_run: bool) -> Result<(), anyhow::Error> {
        let (state, _) = init_state(&self.conf, false).await?;
        let cleanup = StorageCleanup {
            dry_run,
            storage: state.storage,
            db: state.db,
        };
        let confirm = || {
            dialoguer::Confirm::new()
                .with_prompt("Proceed?")
                .interact()
                .unwrap()
        };
        cleanup.run(confirm).await
    }

    async fn migrate(&self, command: MigrateCommand) -> Result<(), anyhow::Error> {
        let (_, db) = init_db(&self.conf).await?;
        match command {
            MigrateCommand::Status => {
                for migration in Migrator::get_migration_with_status(&db).await? {
                    let status = migration.status().to_string().to_lowercase();
                    println!("{status} {}", migration.name());
                }
            }
            MigrateCommand::Up => {
                Migrator::up(&db, None).await?;
            }
        }
        Ok(())
    }

    async fn check(&self) -> Result<(), anyhow::Error> {
        let (_, db) = init_db(&self.conf).await?;
        let pending = Migrator::get_pending_migrations(&db).await?;
        if !pending.is_empty() {
            let names: Vec<_> = pending.iter().map(|m| m.name()).collect();
            anyhow::bail!("Pending migrations: {names:?}");
        }

        let (state, _) = init_state(&self.conf, false).await?;

        // Templates are loaded lazily, so compile every one up front to catch
        // syntax errors before they are hit by a request.
        let templates_dir = release_dir()?.join("templates");
        let mut names = Vec::new();
        collect_template_names(&templates_dir, &templates_dir, &mut names)?;
        for name in &names {
            state
                .template_engine
                .get_template(name)
                .with_context(|| format!("Failed to compile template {name}"))?;
        }
        info!("Templates: {} compiled", names.len());

        let containers = state.storage.list_containers().await?;

        info!("Containers: {containers:?}");

        Ok(())
    }
}

/// Collects the names (relative paths) of all files under `dir`.
fn collect_template_names(
    root: &Path,
    dir: &Path,
    names: &mut Vec<String>,
) -> Result<(), anyhow::Error> {
    let entries = std::fs::read_dir(dir)
        .with_context(|| format!("Failed to read templates directory {}", dir.display()))?;
    for entry in entries {
        let path = entry?.path();
        if path.is_dir() {
            collect_template_names(root, &path, names)?;
        } else {
            let relative = path.strip_prefix(root)?;
            names.push(relative.to_string_lossy().replace('\\', "/"));
        }
    }
    Ok(())
}

#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    init_tracing()?;
    load_env()?;
    let conf = AppConfig::from_env()?;

    let args = CliArgs::parse();
    let cli = Cli { args, conf };
    cli.run().await
}
