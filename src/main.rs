use std::io::Write;

use anyhow::Context;
use clap::{Parser, Subcommand};
use cookie_odyssey::{
    auth::sessions::AuthBackend,
    server::{init_db, init_state},
    storage::StorageCleanup,
};
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

    async fn check(&self) -> Result<(), anyhow::Error> {
        let (state, _) = init_state(&self.conf, false).await?;
        let containers = state.storage.list_containers().await?;

        info!("Containers: {containers:?}");

        Ok(())
    }
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
