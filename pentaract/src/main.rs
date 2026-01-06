use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
    time,
};

use tokio::{sync::mpsc, time};

use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use crate::{
    common::{channels::ClientMessage, db::pool::get_pool, routing::app_state::AppState},
    config::Config,
    server::Server,
    startup::{create_db, create_superuser, init_db},
    storage_manager::StorageManager,
};

mod common;
mod config;
mod errors;
mod models;
mod repositories;
mod routers;
mod schemas;
mod server;
mod services;
mod startup;
mod storage_manager;

#[tokio::main]
async fn main() {
    let config = match Config::new() {
        Ok(config) => config,
        Err(e) => {
            eprintln!("Configuration Error: {}", e);
            eprintln!("\nRequired environment variables:");
            eprintln!("- SUPERUSER_EMAIL: Email for the admin account");
            eprintln!("- SUPERUSER_PASS: Password for the admin account");
            eprintln!("- SECRET_KEY: JWT signing secret");
            eprintln!("- DATABASE_URL: PostgreSQL connection string");
            eprintln!("\nOr set individual database variables:");
            eprintln!("- DATABASE_USER, DATABASE_PASSWORD, DATABASE_NAME");
            eprintln!("- DATABASE_HOST, DATABASE_PORT");
            std::process::exit(1);
        }
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| {
                "pentaract=debug,tower_http=debug,axum::rejection=trace".into()
            }),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let (tx, rx) = mpsc::channel::<ClientMessage>(config.channel_capacity.into());

    // Try to connect to database but continue even if it fails
    let db = match get_pool(
        &config.db_uri,
        config.workers.into(),
        time::Duration::from_secs(60),
    )
    .await
    {
        Ok(db) => {
            tracing::info!("Database connected successfully");
            
            // Initialize database in background
            let db_clone = db.clone();
            let config_clone = config.clone();
            tokio::spawn(async move {
                init_db(&db_clone).await;
                create_superuser(&db_clone, &config_clone).await;
            });
            
            Some(db)
        }
        Err(e) => {
            tracing::error!("Database Connection Error: {}", e);
            let masked_db_url = format!("{}@***", 
                config.db_uri.split('@').next().unwrap_or("unknown"));
            tracing::error!("DATABASE_URL: {}", masked_db_url);
            tracing::warn!("Server will start but database operations will fail");
            
            None
        }
    };

    // Start storage manager if database is available
    if let Some(ref db_connection) = db {
        let config_copy = config.clone();
        tokio::spawn(async move {
            let db = get_pool(
                &config_copy.db_uri,
                config_copy.workers.into(),
                time::Duration::from_secs(30),
            )
            .await
            .unwrap_or_else(|_| {
                tracing::error!("Failed to reconnect to database in storage manager");
                return;
            });
            
            let mut manager = StorageManager::new(rx, db, config_copy);
            tracing::debug!("running manager");
            manager.run().await;
        });
    }

    // Always start the server for Render port detection
    let port = std::env::var("PORT")
        .ok()
        .and_then(|p| p.parse().ok())
        .unwrap_or(config.port);
    
    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), port);
    
    tracing::info!("=== STARTING SERVER ON PORT {} FOR RENDER ===", port);
    tracing::info!("Server will start at: http://{}:{}", addr.ip(), addr.port());
    
    // Create app state with optional database
    let app_state = AppState::new(db, config, tx);
    let shared_state = Arc::new(app_state);
    
    let server = Server::build_server(config.workers.into(), shared_state);

    tracing::info!("Server starting...");
    server.run(&addr).await
}