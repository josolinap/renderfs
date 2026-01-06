use std::{
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
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

    // creating db
    if let (Some(dsn), Some(name)) = (
        config.db_uri_without_dbname.as_deref(),
        config.db_name.as_deref(),
    ) {
        create_db(
            dsn,
            name,
            config.workers.into(),
            time::Duration::from_secs(30),
        )
        .await;
    }

    // Wait a bit for database to be ready (Render might need time)
    tokio::time::sleep(time::Duration::from_secs(5)).await;
    
    // set up connection pool
    let db = match get_pool(
        &config.db_uri,
        config.workers.into(),
        time::Duration::from_secs(60), // Increased timeout
    )
    .await
    {
        Ok(db) => db,
        Err(e) => {
            eprintln!("Database Connection Error: {}", e);
            eprintln!("\nTroubleshooting:");
            eprintln!("1. Check if DATABASE_URL is correct");
            eprintln!("2. Verify database is running and accessible");
            eprintln!("3. Check if database connection limits are reached");
            eprintln!("4. Ensure database SSL/TLS settings are compatible");
            eprintln!("5. DATABASE_URL: {}", &config.db_uri);
            std::process::exit(1);
        }
    };

    // initing db
    init_db(&db).await;

    // creating a superuser
    create_superuser(&db, &config).await;

    // running manager
    let config_copy = config.clone();
    tokio::spawn(async move {
        let db = match get_pool(
            &config_copy.db_uri,
            config_copy.workers.into(),
            time::Duration::from_secs(30),
        )
        .await
        {
            Ok(db) => db,
            Err(e) => {
                tracing::error!("Failed to connect to database in storage manager: {}", e);
                return;
            }
        };
        let mut manager = StorageManager::new(rx, db, config_copy);

        tracing::debug!("running manager");
        manager.run().await;
    });

    let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), config.port);

    let server = {
        let workers = config.workers;
        let app_state = AppState::new(db, config, tx);
        let shared_state = Arc::new(app_state);
        Server::build_server(workers.into(), shared_state)
    };

    server.run(&addr).await
}
