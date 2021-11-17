use sqlx::Postgres;

use super::AppResult;

pub async fn pool() -> AppResult<sqlx::Pool<Postgres>> {
    use sqlx::postgres::PgPoolOptions;
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL not set.");

    let pool = PgPoolOptions::new()
        .max_connections(16)
        .connect(&database_url)
        .await?;

    Ok(pool)
}
