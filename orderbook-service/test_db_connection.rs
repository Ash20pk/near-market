// Quick test to check database connection and orders
use sqlx::PgPool;
use anyhow::Result;

#[tokio::main]
async fn main() -> Result<()> {
    let database_url = std::env::var("DATABASE_URL")
        .expect("DATABASE_URL must be set");

    println!("🔍 Connecting to database...");
    println!("Database URL: {}", database_url);

    let pool = PgPool::connect(&database_url).await?;

    println!("✅ Connected successfully!");

    // Test basic connection
    let result: i32 = sqlx::query_scalar("SELECT 1")
        .fetch_one(&pool)
        .await?;
    println!("✅ Basic query works: {}", result);

    // Check if orders table exists
    let table_exists: bool = sqlx::query_scalar(
        "SELECT EXISTS (SELECT FROM information_schema.tables WHERE table_name = 'orders')"
    )
    .fetch_one(&pool)
    .await?;

    println!("📋 Orders table exists: {}", table_exists);

    if table_exists {
        // Count orders
        let count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM orders")
            .fetch_one(&pool)
            .await?;
        println!("📊 Total orders in database: {}", count);

        // Show recent orders
        let orders: Vec<(String, String, String)> = sqlx::query_as(
            "SELECT order_id::text, market_id, user_account FROM orders ORDER BY created_at DESC LIMIT 5"
        )
        .fetch_all(&pool)
        .await?;

        println!("🔍 Recent orders:");
        for (order_id, market_id, user_account) in orders {
            println!("  - Order: {} | Market: {} | User: {}",
                order_id, market_id, user_account);
        }
    } else {
        println!("❌ Orders table does not exist!");

        // List all tables
        let tables: Vec<(String,)> = sqlx::query_as(
            "SELECT table_name FROM information_schema.tables WHERE table_schema = 'public'"
        )
        .fetch_all(&pool)
        .await?;

        println!("📋 Available tables:");
        for (table_name,) in tables {
            println!("  - {}", table_name);
        }
    }

    Ok(())
}