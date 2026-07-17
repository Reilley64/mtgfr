//! mtgfr CLI: gRPC/health API server, static SPA, and Toasty migrations.

use anyhow::Context;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::Ordering;
use std::time::Duration;
use toasty_cli::{Config, ToastyCli};
use tokio::net::TcpListener;

#[derive(Parser)]
#[command(name = "mtgfr", about = "mtgfr server CLI")]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand)]
enum Commands {
    /// Run the API (default when no subcommand is given): tonic gRPC + Axum health checks
    Serve,
    /// Serve the client SPA (`STATIC_ROOT`, default `./dist`)
    Static,
    /// Toasty schema migrations — pass through (`apply`, `generate`, …)
    Migration {
        #[arg(trailing_var_arg = true, allow_hyphen_values = true, num_args = 1..)]
        args: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        None | Some(Commands::Serve) => {
            run_serve().await;
            Ok(())
        }
        Some(Commands::Static) => {
            run_static().await;
            Ok(())
        }
        Some(Commands::Migration { args }) => run_migration(args).await,
    }
}

async fn run_serve() {
    let settings =
        server::settings::Settings::load().expect("load settings (config/mtgfr.toml or env)");
    let addr = settings.listen_addr();
    let grpc_addr = settings.grpc_listen_addr();
    let listener = TcpListener::bind(&addr).await.expect("bind address");
    let grpc_socket_addr = grpc_addr.parse().expect("valid grpc listen address");

    // Durable store for accounts, sessions, and decks. Schema comes from `just migrate`
    // (Postgres); sqlite tests still use push_schema in `db::connect`.
    let mut db = server::db::connect(&settings.database_url)
        .await
        .expect("connect database (set DATABASE_URL)");

    // Project the card pool into the searchable `catalog_cards` table for the deck builder. Best
    // effort: a failure (e.g. a dev DB the app can't DDL) leaves search empty but the server up.
    if let Err(e) = server::catalog_search::project(&mut db).await {
        eprintln!("catalog projection skipped: {e}");
    }

    let version = settings.version.clone();
    // Tables are born already seeded via Tables.Seed (the BFF owns the pre-game lobby);
    // the registry starts empty. Action traces land at data/actions.<table_id>.toon (gitignored)
    // for post-hoc debugging.
    let state = server::AppState::new(db, Arc::new(settings));
    println!("mtgfr server v{version} health checks on http://{addr}");
    println!("mtgfr gRPC listening on {grpc_addr}");
    println!("action traces: ./data/actions.<table>.toon");

    // Both transports share one drain signal: SIGTERM/Ctrl-C flips `draining`, then blocks on
    // this instance's in-memory tables draining to zero before either server stops accepting.
    let (shutdown_tx, mut http_shutdown_rx) = tokio::sync::watch::channel(false);
    let mut grpc_shutdown_rx = http_shutdown_rx.clone();
    let shutdown_task = {
        let state = state.clone();
        async move {
            await_shutdown_signal(state).await;
            let _ = shutdown_tx.send(true);
        }
    };

    let http_task =
        axum::serve(listener, server::app(state.clone())).with_graceful_shutdown(async move {
            let _ = http_shutdown_rx.changed().await;
        });
    let grpc_task = server::grpc::serve(grpc_socket_addr, state, async move {
        let _ = grpc_shutdown_rx.changed().await;
    });

    let (http_res, grpc_res, ()) = tokio::join!(http_task, grpc_task, shutdown_task);
    http_res.expect("serve http");
    grpc_res.expect("serve grpc");
}

/// On SIGTERM/Ctrl-C: enter drain, then wait until in-memory tables are gone (or kube
/// hits `terminationGracePeriodSeconds` and SIGKILLs).
async fn await_shutdown_signal(state: server::AppState) {
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("install the Ctrl-C handler");
    };
    #[cfg(unix)]
    let terminate = async {
        tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("install the SIGTERM handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {}
        _ = terminate => {}
    }
    state.draining.store(true, Ordering::Relaxed);
    loop {
        let n = server::lock(&state.reg).active_table_count();
        if n == 0 {
            break;
        }
        tokio::time::sleep(Duration::from_secs(5)).await;
    }
}

async fn run_static() {
    let root = static_root();
    let index = root.join("index.html");
    // `.fallback` keeps client routes at 200 (unlike `.not_found_service`).
    let serve_dir = tower_http::services::ServeDir::new(&root)
        .fallback(tower_http::services::ServeFile::new(&index));

    let app = axum::Router::new()
        .route("/health/live", axum::routing::get(static_health_live))
        .fallback_service(serve_dir)
        .layer(tower_http::compression::CompressionLayer::new());

    let addr = static_listen_addr();
    let listener = TcpListener::bind(&addr).await.expect("bind address");
    println!(
        "mtgfr static server listening on http://{addr} (root: {})",
        root.display()
    );
    axum::serve(listener, app).await.expect("serve");
}

fn static_root() -> PathBuf {
    std::env::var("STATIC_ROOT")
        .unwrap_or_else(|_| "./dist".to_string())
        .into()
}

fn static_listen_addr() -> String {
    let host = std::env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let port = std::env::var("PORT").unwrap_or_else(|_| "8080".to_string());
    format!("{host}:{port}")
}

async fn static_health_live() -> &'static str {
    "ok"
}

async fn run_migration(args: Vec<String>) -> anyhow::Result<()> {
    let config = Config::load()?;
    let db_url = std::env::var("DATABASE_URL")
        .context("DATABASE_URL must be set (no localhost default for migrations)")?;

    let mut builder = toasty::Db::builder();
    builder.models(server::db::model_set());
    let db = builder.connect(&db_url).await?;

    let mut argv = vec!["mtgfr".to_string(), "migration".to_string()];
    argv.extend(args);
    ToastyCli::with_config(db, config).parse_from(argv).await?;
    Ok(())
}

#[cfg(test)]
mod cli_tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn defaults_to_serve_when_no_subcommand() {
        let cli = Cli::try_parse_from(["mtgfr"]).unwrap();
        assert!(cli.command.is_none());
    }

    #[test]
    fn parses_serve_subcommand() {
        let cli = Cli::try_parse_from(["mtgfr", "serve"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Serve)));
    }

    #[test]
    fn parses_static_subcommand() {
        let cli = Cli::try_parse_from(["mtgfr", "static"]).unwrap();
        assert!(matches!(cli.command, Some(Commands::Static)));
    }

    #[test]
    fn parses_migration_passthrough_args() {
        let cli = Cli::try_parse_from(["mtgfr", "migration", "apply"]).unwrap();
        let Commands::Migration { args } = cli.command.unwrap() else {
            panic!("expected migration subcommand");
        };
        assert_eq!(args, ["apply"]);
    }
}

#[cfg(test)]
mod static_tests {
    use super::*;

    static ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn with_env<R>(key: &str, value: Option<&str>, f: impl FnOnce() -> R) -> R {
        let saved = std::env::var(key).ok();
        match value {
            Some(v) => unsafe { std::env::set_var(key, v) },
            None => unsafe { std::env::remove_var(key) },
        }
        let result = f();
        match saved {
            Some(v) => unsafe { std::env::set_var(key, v) },
            None => unsafe { std::env::remove_var(key) },
        }
        result
    }

    #[test]
    fn static_root_defaults_to_dist_when_unset() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        with_env("STATIC_ROOT", None, || {
            assert_eq!(static_root(), PathBuf::from("./dist"));
        });
    }

    #[test]
    fn static_root_honors_the_env_override() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        with_env("STATIC_ROOT", Some("/app/dist"), || {
            assert_eq!(static_root(), PathBuf::from("/app/dist"));
        });
    }

    #[test]
    fn listen_addr_defaults_to_0_0_0_0_8080() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        with_env("HOST", None, || {
            with_env("PORT", None, || {
                assert_eq!(static_listen_addr(), "0.0.0.0:8080");
            });
        });
    }

    #[test]
    fn listen_addr_honors_env_overrides() {
        let _guard = ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        with_env("HOST", Some("127.0.0.1"), || {
            with_env("PORT", Some("9090"), || {
                assert_eq!(static_listen_addr(), "127.0.0.1:9090");
            });
        });
    }

    #[tokio::test]
    async fn health_live_reports_ok() {
        assert_eq!(static_health_live().await, "ok");
    }

    #[tokio::test]
    async fn health_live_is_reachable_ahead_of_the_spa_fallback() {
        use axum::body::{Body, to_bytes};
        use axum::http::{Request, StatusCode};
        use tower::ServiceExt;

        let dir = std::env::temp_dir().join(format!("mtgfr-static-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).expect("create temp static root");
        let index = dir.join("index.html");
        std::fs::write(&index, "<html>spa</html>").expect("write index.html");

        let serve_dir = tower_http::services::ServeDir::new(&dir)
            .fallback(tower_http::services::ServeFile::new(&index));
        let app = axum::Router::new()
            .route("/health/live", axum::routing::get(static_health_live))
            .fallback_service(serve_dir);

        let res = app
            .oneshot(
                Request::builder()
                    .uri("/health/live")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(res.status(), StatusCode::OK);
        let body = to_bytes(res.into_body(), usize::MAX).await.unwrap();
        assert_eq!(&body[..], b"ok");

        let _ = std::fs::remove_dir_all(&dir);
    }
}
