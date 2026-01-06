use std::time::Duration;

use sqlx::{postgres::PgPoolOptions, PgPool};

pub async fn get_pool(dsn: &str, max_connection: u32, timeout: Duration) -> Result<PgPool, sqlx::Error> {
    // Retry connection multiple times with exponential backoff
    let mut retries = 5;
    let mut base_delay = Duration::from_secs(2);
    
    while retries > 0 {
        match PgPoolOptions::new()
            .max_connections(max_connection)
            .acquire_timeout(timeout)
            .min_connections(1)
            .max_lifetime(Duration::from_secs(30 * 60))
            .idle_timeout(Duration::from_secs(10 * 60))
            .connect(dsn)
            .await
        {
            Ok(pool) => {
                tracing::debug!("established connection with database");
                return Ok(pool);
            }
            Err(e) => {
                tracing::warn!("database connection attempt failed ({} retries left): {}", retries - 1, e);
                
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
