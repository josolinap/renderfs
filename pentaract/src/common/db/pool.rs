use std::time::Duration;

use sqlx::{postgres::PgPoolOptions, PgPool};

pub async fn get_pool(dsn: &str, max_connection: u32, timeout: Duration) -> Result<PgPool, sqlx::Error> {
    // Retry connection multiple times with exponential backoff
    let mut retries = 8;
    let mut base_delay = Duration::from_secs(3);
    
    // Add connection pool options optimized for Render
    while retries > 0 {
        match PgPoolOptions::new()
            .max_connections(max_connection.min(4)) // Limit connections for Render free tier
            .acquire_timeout(Duration::from_secs(30))
            .min_connections(0) // Don't require initial connections
            .max_lifetime(Duration::from_secs(10 * 60))
            .idle_timeout(Duration::from_secs(5 * 60))
            .connect(dsn)
            .await
        {
            Ok(pool) => {
                tracing::debug!("established connection with database");
                return Ok(pool);
            }
            Err(e) => {
                tracing::warn!("database connection attempt failed ({} retries left): {}", retries - 1, e);
                tracing::info!("DATABASE_URL: {}", dsn.split('@').next().unwrap_or("hidden").to_string() + "@***");
                
                if retries == 1 {
                    return Err(e);
                }
                
                tokio::time::sleep(base_delay).await;
                base_delay *= 2;
                retries -= 1;
            }
        }
    }
    
    Err(sqlx::Error::Configuration("Failed to connect to database after multiple retries".into()))
}
