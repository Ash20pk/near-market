// Example PostgreSQL storage implementation to replace in-memory Database
// This shows how to integrate with Supabase PostgreSQL

use sqlx::{PgPool, Row};
use uuid::Uuid;
use anyhow::{anyhow, Result};
use chrono::Utc;
use std::collections::HashMap;

use crate::types::{Order, Trade, SettlementStatus, CollateralBalance, CollateralReservation, OrderStatus};

pub struct PostgresDatabase {
    pool: PgPool,
}

impl PostgresDatabase {
    pub async fn new(database_url: &str) -> Result<Self> {
        let pool = PgPool::connect(database_url).await?;

        // Run any migrations if needed
        sqlx::migrate!("./migrations").run(&pool).await?;

        Ok(Self { pool })
    }

    // ================================
    // ORDER OPERATIONS
    // ================================

    pub async fn insert_order(&self, order: &Order) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO orders (
                order_id, market_id, condition_id, user_account, outcome,
                side, order_type, price, original_size, remaining_size,
                filled_size, status, created_at, expires_at, solver_account
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            "#,
            order.order_id,
            order.market_id,
            order.condition_id,
            order.user_account,
            order.outcome as i16,
            match order.side {
                crate::types::OrderSide::Buy => "Buy",
                crate::types::OrderSide::Sell => "Sell",
            },
            match order.order_type {
                crate::types::OrderType::Limit => "Limit",
                crate::types::OrderType::Market => "Market",
            },
            order.price as i64,
            sqlx::types::BigDecimal::from(order.original_size),
            sqlx::types::BigDecimal::from(order.remaining_size),
            sqlx::types::BigDecimal::from(order.filled_size),
            match order.status {
                OrderStatus::Pending => "Pending",
                OrderStatus::PartiallyFilled => "PartiallyFilled",
                OrderStatus::Filled => "Filled",
                OrderStatus::Cancelled => "Cancelled",
                OrderStatus::Expired => "Expired",
                OrderStatus::Failed => "Failed",
            },
            order.created_at,
            order.expires_at,
            order.solver_account
        )
        .execute(&self.pool)
        .await?;

        // Trigger market stats update
        self.update_market_stats(&order.market_id, order.outcome).await?;

        Ok(())
    }

    pub async fn update_order(&self, order: &Order) -> Result<()> {
        sqlx::query!(
            r#"
            UPDATE orders SET
                remaining_size = $1,
                filled_size = $2,
                status = $3
            WHERE order_id = $4
            "#,
            sqlx::types::BigDecimal::from(order.remaining_size),
            sqlx::types::BigDecimal::from(order.filled_size),
            match order.status {
                OrderStatus::Pending => "Pending",
                OrderStatus::PartiallyFilled => "PartiallyFilled",
                OrderStatus::Filled => "Filled",
                OrderStatus::Cancelled => "Cancelled",
                OrderStatus::Expired => "Expired",
                OrderStatus::Failed => "Failed",
            },
            order.order_id
        )
        .execute(&self.pool)
        .await?;

        // Trigger market stats update
        self.update_market_stats(&order.market_id, order.outcome).await?;

        Ok(())
    }

    pub async fn get_order(&self, order_id: Uuid) -> Result<Option<Order>> {
        let row = sqlx::query!(
            "SELECT * FROM orders WHERE order_id = $1",
            order_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| Order {
            order_id: r.order_id,
            market_id: r.market_id,
            condition_id: r.condition_id,
            user_account: r.user_account,
            outcome: r.outcome as u8,
            side: match r.side.as_str() {
                "Buy" => crate::types::OrderSide::Buy,
                "Sell" => crate::types::OrderSide::Sell,
                _ => crate::types::OrderSide::Buy, // Default fallback
            },
            order_type: match r.order_type.as_str() {
                "Limit" => crate::types::OrderType::Limit,
                "Market" => crate::types::OrderType::Market,
                _ => crate::types::OrderType::Limit,
            },
            price: r.price as u64,
            original_size: r.original_size.to_string().parse().unwrap_or(0),
            remaining_size: r.remaining_size.to_string().parse().unwrap_or(0),
            filled_size: r.filled_size.to_string().parse().unwrap_or(0),
            status: match r.status.as_str() {
                "Pending" => OrderStatus::Pending,
                "PartiallyFilled" => OrderStatus::PartiallyFilled,
                "Filled" => OrderStatus::Filled,
                "Cancelled" => OrderStatus::Cancelled,
                "Expired" => OrderStatus::Expired,
                "Failed" => OrderStatus::Failed,
                _ => OrderStatus::Pending,
            },
            created_at: r.created_at,
            expires_at: r.expires_at,
            solver_account: r.solver_account,
        }))
    }

    pub async fn get_active_orders(&self) -> Result<Vec<Order>> {
        let rows = sqlx::query!(
            r#"
            SELECT * FROM orders
            WHERE status IN ('Pending', 'PartiallyFilled')
            ORDER BY created_at ASC
            "#
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| Order {
            order_id: r.order_id,
            market_id: r.market_id,
            condition_id: r.condition_id,
            user_account: r.user_account,
            outcome: r.outcome as u8,
            side: match r.side.as_str() {
                "Buy" => crate::types::OrderSide::Buy,
                "Sell" => crate::types::OrderSide::Sell,
                _ => crate::types::OrderSide::Buy,
            },
            order_type: match r.order_type.as_str() {
                "Limit" => crate::types::OrderType::Limit,
                "Market" => crate::types::OrderType::Market,
                _ => crate::types::OrderType::Limit,
            },
            price: r.price as u64,
            original_size: r.original_size.to_string().parse().unwrap_or(0),
            remaining_size: r.remaining_size.to_string().parse().unwrap_or(0),
            filled_size: r.filled_size.to_string().parse().unwrap_or(0),
            status: match r.status.as_str() {
                "Pending" => OrderStatus::Pending,
                "PartiallyFilled" => OrderStatus::PartiallyFilled,
                "Filled" => OrderStatus::Filled,
                "Cancelled" => OrderStatus::Cancelled,
                "Expired" => OrderStatus::Expired,
                "Failed" => OrderStatus::Failed,
                _ => OrderStatus::Pending,
            },
            created_at: r.created_at,
            expires_at: r.expires_at,
            solver_account: r.solver_account,
        }).collect())
    }

    // ================================
    // ORDERBOOK QUERIES (This fixes ask visibility!)
    // ================================

    pub async fn get_orderbook_bids(&self, market_id: &str, outcome: u8, limit: i32) -> Result<Vec<crate::types::PriceLevel>> {
        let rows = sqlx::query!(
            r#"
            SELECT price, SUM(remaining_size) as total_size, COUNT(*) as order_count
            FROM orders
            WHERE market_id = $1 AND outcome = $2 AND side = 'Buy'
              AND status IN ('Pending', 'PartiallyFilled')
            GROUP BY price
            ORDER BY price DESC
            LIMIT $3
            "#,
            market_id,
            outcome as i16,
            limit as i64
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| crate::types::PriceLevel {
            price: r.price as u64,
            size: r.total_size.to_string().parse().unwrap_or(0),
            order_count: r.order_count.unwrap_or(0) as u32,
        }).collect())
    }

    pub async fn get_orderbook_asks(&self, market_id: &str, outcome: u8, limit: i32) -> Result<Vec<crate::types::PriceLevel>> {
        let rows = sqlx::query!(
            r#"
            SELECT price, SUM(remaining_size) as total_size, COUNT(*) as order_count
            FROM orders
            WHERE market_id = $1 AND outcome = $2 AND side = 'Sell'
              AND status IN ('Pending', 'PartiallyFilled')
            GROUP BY price
            ORDER BY price ASC
            LIMIT $3
            "#,
            market_id,
            outcome as i16,
            limit as i64
        )
        .fetch_all(&self.pool)
        .await?;

        Ok(rows.into_iter().map(|r| crate::types::PriceLevel {
            price: r.price as u64,
            size: r.total_size.to_string().parse().unwrap_or(0),
            order_count: r.order_count.unwrap_or(0) as u32,
        }).collect())
    }

    // ================================
    // MARKET STATS (This fixes N/A values!)
    // ================================

    pub async fn get_market_stats(&self, market_id: &str, outcome: u8) -> Result<Option<crate::types::MarketPrice>> {
        let row = sqlx::query!(
            r#"
            SELECT best_bid, best_ask, mid_price, last_price, updated_at
            FROM market_stats
            WHERE market_id = $1 AND outcome = $2
            "#,
            market_id,
            outcome as i16
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| crate::types::MarketPrice {
            market_id: market_id.to_string(),
            outcome,
            bid: r.best_bid.map(|b| b as u64),
            ask: r.best_ask.map(|a| a as u64),
            mid: r.mid_price.map(|m| m as u64),
            last: r.last_price.map(|l| l as u64),
            timestamp: r.updated_at,
        }))
    }

    async fn update_market_stats(&self, market_id: &str, outcome: u8) -> Result<()> {
        sqlx::query!(
            "SELECT update_market_stats($1, $2)",
            market_id,
            outcome as i16
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ================================
    // TRADE OPERATIONS
    // ================================

    pub async fn insert_trade(&self, trade: &Trade) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO trades (
                trade_id, market_id, condition_id, maker_order_id, taker_order_id,
                maker_account, taker_account, maker_side, taker_side, outcome,
                price, size, trade_type, executed_at, settlement_status
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
            "#,
            trade.trade_id,
            trade.market_id,
            trade.condition_id,
            trade.maker_order_id,
            trade.taker_order_id,
            trade.maker_account,
            trade.taker_account,
            match trade.maker_side {
                crate::types::OrderSide::Buy => "Buy",
                crate::types::OrderSide::Sell => "Sell",
            },
            match trade.taker_side {
                crate::types::OrderSide::Buy => "Buy",
                crate::types::OrderSide::Sell => "Sell",
            },
            trade.outcome as i16,
            trade.price as i64,
            sqlx::types::BigDecimal::from(trade.size),
            match trade.trade_type {
                crate::types::TradeType::DirectMatch => "DirectMatch",
                crate::types::TradeType::Minting => "Minting",
                crate::types::TradeType::Burning => "Burning",
            },
            trade.executed_at,
            match trade.settlement_status {
                SettlementStatus::Pending => "Pending",
                SettlementStatus::Settling => "Settling",
                SettlementStatus::Settled => "Settled",
                SettlementStatus::Failed => "Failed",
            }
        )
        .execute(&self.pool)
        .await?;

        // Trigger market stats update
        self.update_market_stats(&trade.market_id, trade.outcome).await?;

        Ok(())
    }

    // ================================
    // COLLATERAL OPERATIONS
    // ================================

    pub async fn get_collateral_balance(&self, account_id: &str, market_id: &str) -> Result<Option<CollateralBalance>> {
        let row = sqlx::query!(
            r#"
            SELECT * FROM collateral_balances
            WHERE account_id = $1 AND market_id = $2
            "#,
            account_id,
            market_id
        )
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| CollateralBalance {
            account_id: r.account_id,
            market_id: r.market_id,
            available_balance: r.available_balance.to_string().parse().unwrap_or(0),
            reserved_balance: r.reserved_balance.to_string().parse().unwrap_or(0),
            position_balance: r.position_balance.to_string().parse().unwrap_or(0),
            total_deposited: r.total_deposited.to_string().parse().unwrap_or(0),
            total_withdrawn: r.total_withdrawn.to_string().parse().unwrap_or(0),
            last_updated: r.last_updated,
        }))
    }

    pub async fn update_collateral_balance(&self, balance: &CollateralBalance) -> Result<()> {
        sqlx::query!(
            r#"
            INSERT INTO collateral_balances (
                account_id, market_id, available_balance, reserved_balance,
                position_balance, total_deposited, total_withdrawn, last_updated
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (account_id, market_id)
            DO UPDATE SET
                available_balance = EXCLUDED.available_balance,
                reserved_balance = EXCLUDED.reserved_balance,
                position_balance = EXCLUDED.position_balance,
                total_deposited = EXCLUDED.total_deposited,
                total_withdrawn = EXCLUDED.total_withdrawn,
                last_updated = EXCLUDED.last_updated
            "#,
            balance.account_id,
            balance.market_id,
            sqlx::types::BigDecimal::from(balance.available_balance),
            sqlx::types::BigDecimal::from(balance.reserved_balance),
            sqlx::types::BigDecimal::from(balance.position_balance),
            sqlx::types::BigDecimal::from(balance.total_deposited),
            sqlx::types::BigDecimal::from(balance.total_withdrawn),
            balance.last_updated
        )
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    // ================================
    // REAL-TIME SUBSCRIPTIONS (Supabase feature)
    // ================================

    pub async fn subscribe_to_orderbook_changes(&self, market_id: &str) -> Result<()> {
        // This would integrate with Supabase real-time subscriptions
        // For now, this is a placeholder showing the concept
        println!("Setting up real-time subscription for market: {}", market_id);
        Ok(())
    }
}

// ================================
// HELPER FUNCTIONS FOR INTEGRATION
// ================================

// Helper to get database connection from environment
pub async fn create_database_connection() -> Result<PostgresDatabase> {
    let database_url = std::env::var("DATABASE_URL")
        .map_err(|_| anyhow!("DATABASE_URL not set"))?;

    PostgresDatabase::new(&database_url).await
}

// Helper to migrate from in-memory to PostgreSQL
pub async fn migrate_existing_data(
    old_db: &crate::storage::Database,
    new_db: &PostgresDatabase
) -> Result<()> {
    // Get all existing orders
    let orders = old_db.get_active_orders().await?;

    // Insert into PostgreSQL
    for order in orders {
        new_db.insert_order(&order).await?;
    }

    println!("Migrated {} orders to PostgreSQL", orders.len());
    Ok(())
}