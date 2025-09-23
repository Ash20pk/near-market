// Simplified PostgreSQL implementation without compile-time query verification
// Drop-in replacement for in-memory Database preserving exact same interface

use sqlx::{PgPool, Row, postgres::PgPoolOptions};
use uuid::Uuid;
use anyhow::{anyhow, Result};
use chrono::Utc;
use tracing::info;

use crate::types::{
    Order, Trade, SettlementStatus, CollateralBalance, CollateralReservation,
    OrderStatus, OrderSide, OrderType, TradeType, OrderbookSnapshot, MarketPrice, PriceLevel
};

pub struct SimplePostgresDatabase {
    pool: PgPool,
}

impl SimplePostgresDatabase {
    pub async fn new() -> Result<Self> {
        let database_url = std::env::var("DATABASE_URL")
            .map_err(|_| anyhow!("DATABASE_URL environment variable not set"))?;

        info!("🐘 Connecting to PostgreSQL database...");

        // Configure pool to prevent prepared statement conflicts
        let pool = PgPoolOptions::new()
            .max_connections(10)
            .min_connections(1)
            .acquire_timeout(std::time::Duration::from_secs(30))
            .idle_timeout(std::time::Duration::from_secs(300))
            .max_lifetime(std::time::Duration::from_secs(1800))
            .connect(&database_url).await
            .map_err(|e| anyhow!("Failed to connect to database: {}", e))?;

        // Test the connection
        let _test_row = sqlx::query("SELECT 1 as test")
            .fetch_one(&pool)
            .await
            .map_err(|e| anyhow!("Database connection test failed: {}", e))?;

        info!("✅ PostgreSQL database connected successfully");
        Ok(Self { pool })
    }

    pub async fn new_test() -> Result<Self> {
        Self::new().await
    }

    // ================================
    // ORDER OPERATIONS (Exact same interface as Database)
    // ================================

    pub async fn insert_order(&self, order: &Order) -> Result<()> {
        let query = r#"
            INSERT INTO orders (
                order_id, market_id, condition_id, user_account, outcome,
                side, order_type, price, original_size, remaining_size,
                filled_size, status, created_at, expires_at, solver_account
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
        "#;

        sqlx::query(query)
            .bind(order.order_id)
            .bind(&order.market_id)
            .bind(&order.condition_id)
            .bind(&order.user_account)
            .bind(order.outcome as i16)
            .bind(self.order_side_to_string(&order.side))
            .bind(self.order_type_to_string(&order.order_type))
            .bind(order.price as i64)
            .bind(order.original_size as i64) // Simplified - convert u128 to i64 for now
            .bind(order.remaining_size as i64)
            .bind(order.filled_size as i64)
            .bind(self.order_status_to_string(&order.status))
            .bind(order.created_at)
            .bind(order.expires_at)
            .bind(&order.solver_account)
            .execute(&self.pool)
            .await?;

        // Update market stats if the function exists
        let _ = sqlx::query("SELECT update_market_stats($1, $2)")
            .bind(&order.market_id)
            .bind(order.outcome as i16)
            .execute(&self.pool)
            .await; // Ignore errors for now

        info!("📝 Inserted order {} for market {}", order.order_id, order.market_id);
        Ok(())
    }

    pub async fn update_order(&self, order: &Order) -> Result<()> {
        let query = r#"
            UPDATE orders SET
                remaining_size = $1,
                filled_size = $2,
                status = $3
            WHERE order_id = $4
        "#;

        let result = sqlx::query(query)
            .bind(order.remaining_size as i64)
            .bind(order.filled_size as i64)
            .bind(self.order_status_to_string(&order.status))
            .bind(order.order_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("Order {} not found for update", order.order_id));
        }

        info!("📝 Updated order {} status: {:?}", order.order_id, order.status);
        Ok(())
    }

    pub async fn get_order(&self, order_id: Uuid) -> Result<Option<Order>> {
        let query = "SELECT * FROM orders WHERE order_id = $1";

        let row = sqlx::query(query)
            .bind(order_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| self.row_to_order(r)))
    }

    pub async fn get_active_orders(&self) -> Result<Vec<Order>> {
        let query = r#"
            SELECT * FROM orders
            WHERE status IN ('Pending', 'PartiallyFilled')
            ORDER BY created_at ASC
        "#;

        let rows = sqlx::query(query)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().map(|r| self.row_to_order(r)).collect())
    }

    pub async fn get_expired_orders(&self) -> Result<Vec<Order>> {
        let now = Utc::now();
        let query = r#"
            SELECT * FROM orders
            WHERE expires_at < $1
              AND status IN ('Pending', 'PartiallyFilled')
            ORDER BY expires_at ASC
        "#;

        let rows = sqlx::query(query)
            .bind(now)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().map(|r| self.row_to_order(r)).collect())
    }

    // ================================
    // ENHANCED ORDERBOOK QUERIES (The key improvement!)
    // ================================

    pub async fn get_orderbook_snapshot(&self, market_id: &str, outcome: u8) -> Result<Option<OrderbookSnapshot>> {
        // Get bids (buy orders, highest price first)
        let bid_query = r#"
            SELECT price, SUM(remaining_size) as total_size, COUNT(*) as order_count
            FROM orders
            WHERE market_id = $1 AND outcome = $2 AND side = 'Buy'
              AND status IN ('Pending', 'PartiallyFilled')
            GROUP BY price
            ORDER BY price DESC
            LIMIT 20
        "#;

        let bid_rows = sqlx::query(bid_query)
            .bind(market_id)
            .bind(outcome as i16)
            .fetch_all(&self.pool)
            .await?;

        // Get asks (sell orders, lowest price first)
        let ask_query = r#"
            SELECT price, SUM(remaining_size) as total_size, COUNT(*) as order_count
            FROM orders
            WHERE market_id = $1 AND outcome = $2 AND side = 'Sell'
              AND status IN ('Pending', 'PartiallyFilled')
            GROUP BY price
            ORDER BY price ASC
            LIMIT 20
        "#;

        let ask_rows = sqlx::query(ask_query)
            .bind(market_id)
            .bind(outcome as i16)
            .fetch_all(&self.pool)
            .await?;

        // Get last trade price
        let last_trade_query = r#"
            SELECT price FROM trades
            WHERE market_id = $1 AND outcome = $2
            ORDER BY executed_at DESC
            LIMIT 1
        "#;

        let last_trade_row = sqlx::query(last_trade_query)
            .bind(market_id)
            .bind(outcome as i16)
            .fetch_optional(&self.pool)
            .await?;

        let bids: Vec<PriceLevel> = bid_rows.into_iter().map(|r| PriceLevel {
            price: r.get::<i64, _>("price") as u64,
            size: r.get::<i64, _>("total_size") as u128,
            order_count: r.get::<i64, _>("order_count") as u32,
        }).collect();

        let asks: Vec<PriceLevel> = ask_rows.into_iter().map(|r| PriceLevel {
            price: r.get::<i64, _>("price") as u64,
            size: r.get::<i64, _>("total_size") as u128,
            order_count: r.get::<i64, _>("order_count") as u32,
        }).collect();

        // Only return snapshot if there's some data
        if bids.is_empty() && asks.is_empty() {
            return Ok(None);
        }

        info!("📊 Retrieved orderbook snapshot from PostgreSQL: {} bids, {} asks", bids.len(), asks.len());

        Ok(Some(OrderbookSnapshot {
            market_id: market_id.to_string(),
            outcome,
            bids,
            asks,
            last_trade_price: last_trade_row.map(|r| r.get::<i64, _>("price") as u64),
            timestamp: Utc::now(),
        }))
    }

    pub async fn get_market_price(&self, market_id: &str, outcome: u8) -> Result<Option<MarketPrice>> {
        // Try to get from market_stats table if it exists
        let stats_query = r#"
            SELECT best_bid, best_ask, mid_price, last_price, updated_at
            FROM market_stats
            WHERE market_id = $1 AND outcome = $2
        "#;

        if let Ok(row) = sqlx::query(stats_query)
            .bind(market_id)
            .bind(outcome as i16)
            .fetch_optional(&self.pool)
            .await
        {
            if let Some(r) = row {
                return Ok(Some(MarketPrice {
                    market_id: market_id.to_string(),
                    outcome,
                    bid: r.get::<Option<i64>, _>("best_bid").map(|b| b as u64),
                    ask: r.get::<Option<i64>, _>("best_ask").map(|a| a as u64),
                    mid: r.get::<Option<i64>, _>("mid_price").map(|m| m as u64),
                    last: r.get::<Option<i64>, _>("last_price").map(|l| l as u64),
                    timestamp: r.get("updated_at"),
                }));
            }
        }

        // Fallback: calculate from current orders
        let snapshot = self.get_orderbook_snapshot(market_id, outcome).await?;
        match snapshot {
            Some(s) => {
                let bid = s.bids.first().map(|b| b.price);
                let ask = s.asks.first().map(|a| a.price);
                let mid = if let (Some(b), Some(a)) = (bid, ask) {
                    Some((b + a) / 2)
                } else {
                    None
                };

                Ok(Some(MarketPrice {
                    market_id: market_id.to_string(),
                    outcome,
                    bid,
                    ask,
                    mid,
                    last: s.last_trade_price,
                    timestamp: Utc::now(),
                }))
            }
            None => Ok(None),
        }
    }

    // ================================
    // TRADE OPERATIONS
    // ================================

    pub async fn insert_trade(&self, trade: &Trade) -> Result<()> {
        let query = r#"
            INSERT INTO trades (
                trade_id, market_id, condition_id, maker_order_id, taker_order_id,
                maker_account, taker_account, maker_side, taker_side, outcome,
                price, size, trade_type, executed_at, settlement_status, settlement_tx_hash
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
        "#;

        sqlx::query(query)
            .bind(trade.trade_id)
            .bind(&trade.market_id)
            .bind(&trade.condition_id)
            .bind(trade.maker_order_id)
            .bind(trade.taker_order_id)
            .bind(&trade.maker_account)
            .bind(&trade.taker_account)
            .bind(self.order_side_to_string(&trade.maker_side))
            .bind(self.order_side_to_string(&trade.taker_side))
            .bind(trade.outcome as i16)
            .bind(trade.price as i64)
            .bind(trade.size as i64)
            .bind(self.trade_type_to_string(&trade.trade_type))
            .bind(trade.executed_at)
            .bind(self.settlement_status_to_string(&trade.settlement_status))
            .bind(&trade.settlement_tx_hash)
            .execute(&self.pool)
            .await?;

        info!("💰 Inserted trade {} for market {}", trade.trade_id, trade.market_id);
        Ok(())
    }

    pub async fn update_trade_settlement_status(
        &self,
        trade_id: Uuid,
        status: SettlementStatus,
        tx_hash: Option<String>,
    ) -> Result<()> {
        let query = r#"
            UPDATE trades SET
                settlement_status = $1,
                settlement_tx_hash = $2
            WHERE trade_id = $3
        "#;

        let result = sqlx::query(query)
            .bind(self.settlement_status_to_string(&status))
            .bind(tx_hash)
            .bind(trade_id)
            .execute(&self.pool)
            .await?;

        if result.rows_affected() == 0 {
            return Err(anyhow!("Trade {} not found for settlement update", trade_id));
        }

        Ok(())
    }

    pub async fn get_failed_trades(&self) -> Result<Vec<Trade>> {
        let query = r#"
            SELECT * FROM trades
            WHERE settlement_status = 'Failed'
            ORDER BY executed_at DESC
        "#;

        let rows = sqlx::query(query)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().map(|r| self.row_to_trade(r)).collect())
    }

    // ================================
    // TEST-ONLY METHODS (Preserving exact interface)
    // ================================

    pub async fn count_settled_trades(&self) -> Result<usize> {
        let query = "SELECT COUNT(*) as count FROM trades WHERE settlement_status = 'Settled'";
        let row = sqlx::query(query).fetch_one(&self.pool).await?;
        Ok(row.get::<i64, _>("count") as usize)
    }

    pub async fn count_failed_trades(&self) -> Result<usize> {
        let query = "SELECT COUNT(*) as count FROM trades WHERE settlement_status = 'Failed'";
        let row = sqlx::query(query).fetch_one(&self.pool).await?;
        Ok(row.get::<i64, _>("count") as usize)
    }

    pub async fn count_pending_trades(&self) -> Result<usize> {
        let query = "SELECT COUNT(*) as count FROM trades WHERE settlement_status = 'Pending'";
        let row = sqlx::query(query).fetch_one(&self.pool).await?;
        Ok(row.get::<i64, _>("count") as usize)
    }

    pub async fn get_trades_for_market(&self, market_id: &str) -> Result<Vec<Trade>> {
        let query = "SELECT * FROM trades WHERE market_id = $1 ORDER BY executed_at DESC";
        let rows = sqlx::query(query)
            .bind(market_id)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().map(|r| self.row_to_trade(r)).collect())
    }

    pub async fn get_settled_trades_for_condition(&self, condition_id: &str) -> Result<Vec<Trade>> {
        let query = r#"
            SELECT * FROM trades
            WHERE condition_id = $1 AND settlement_status = 'Settled'
            ORDER BY executed_at DESC
        "#;

        let rows = sqlx::query(query)
            .bind(condition_id)
            .fetch_all(&self.pool)
            .await?;

        Ok(rows.into_iter().map(|r| self.row_to_trade(r)).collect())
    }

    pub async fn get_trade_settlement_status(&self, trade_id: Uuid) -> Result<SettlementStatus> {
        let query = "SELECT settlement_status FROM trades WHERE trade_id = $1";
        let row = sqlx::query(query)
            .bind(trade_id)
            .fetch_optional(&self.pool)
            .await?;

        match row {
            Some(r) => Ok(self.string_to_settlement_status(&r.get::<String, _>("settlement_status"))),
            None => Ok(SettlementStatus::Failed),
        }
    }

    // ================================
    // COLLATERAL OPERATIONS (Simplified)
    // ================================

    pub async fn get_collateral_balance(&self, account_id: &str, market_id: &str) -> Result<Option<CollateralBalance>> {
        let query = "SELECT * FROM collateral_balances WHERE account_id = $1 AND market_id = $2";
        let row = sqlx::query(query)
            .bind(account_id)
            .bind(market_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| CollateralBalance {
            account_id: r.get("account_id"),
            market_id: r.get("market_id"),
            available_balance: r.get::<i64, _>("available_balance") as u128,
            reserved_balance: r.get::<i64, _>("reserved_balance") as u128,
            position_balance: r.get::<i64, _>("position_balance") as u128,
            total_deposited: r.get::<i64, _>("total_deposited") as u128,
            total_withdrawn: r.get::<i64, _>("total_withdrawn") as u128,
            last_updated: r.get("last_updated"),
        }))
    }

    pub async fn update_collateral_balance(&self, balance: &CollateralBalance) -> Result<()> {
        let query = r#"
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
        "#;

        sqlx::query(query)
            .bind(&balance.account_id)
            .bind(&balance.market_id)
            .bind(balance.available_balance as i64)
            .bind(balance.reserved_balance as i64)
            .bind(balance.position_balance as i64)
            .bind(balance.total_deposited as i64)
            .bind(balance.total_withdrawn as i64)
            .bind(balance.last_updated)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn store_collateral_reservation(&self, reservation: &CollateralReservation) -> Result<()> {
        let query = r#"
            INSERT INTO collateral_reservations (
                order_id, reservation_id, account_id, market_id, reserved_amount,
                max_loss, side, price, size, created_at
            ) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
        "#;

        sqlx::query(query)
            .bind(reservation.order_id)
            .bind(reservation.reservation_id)
            .bind(&reservation.account_id)
            .bind(&reservation.market_id)
            .bind(reservation.reserved_amount as i64)
            .bind(reservation.max_loss as i64)
            .bind(self.order_side_to_string(&reservation.side))
            .bind(reservation.price as i64)
            .bind(reservation.size as i64)
            .bind(reservation.created_at)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    pub async fn get_collateral_reservation(&self, order_id: Uuid) -> Result<Option<CollateralReservation>> {
        let query = "SELECT * FROM collateral_reservations WHERE order_id = $1";
        let row = sqlx::query(query)
            .bind(order_id)
            .fetch_optional(&self.pool)
            .await?;

        Ok(row.map(|r| CollateralReservation {
            reservation_id: r.get("reservation_id"),
            account_id: r.get("account_id"),
            market_id: r.get("market_id"),
            order_id: r.get("order_id"),
            reserved_amount: r.get::<i64, _>("reserved_amount") as u128,
            max_loss: r.get::<i64, _>("max_loss") as u128,
            side: self.string_to_order_side(&r.get::<String, _>("side")),
            price: r.get::<i64, _>("price") as u64,
            size: r.get::<i64, _>("size") as u128,
            created_at: r.get("created_at"),
        }))
    }

    pub async fn remove_collateral_reservation(&self, order_id: Uuid) -> Result<()> {
        let query = "DELETE FROM collateral_reservations WHERE order_id = $1";
        sqlx::query(query)
            .bind(order_id)
            .execute(&self.pool)
            .await?;

        Ok(())
    }

    // ================================
    // CONVERSION HELPERS
    // ================================

    fn row_to_order(&self, r: sqlx::postgres::PgRow) -> Order {
        Order {
            order_id: r.get("order_id"),
            market_id: r.get("market_id"),
            condition_id: r.get("condition_id"),
            user_account: r.get("user_account"),
            outcome: r.get::<i16, _>("outcome") as u8,
            side: self.string_to_order_side(&r.get::<String, _>("side")),
            order_type: self.string_to_order_type(&r.get::<String, _>("order_type")),
            price: r.get::<i64, _>("price") as u64,
            original_size: r.get::<i64, _>("original_size") as u128,
            remaining_size: r.get::<i64, _>("remaining_size") as u128,
            filled_size: r.get::<i64, _>("filled_size") as u128,
            status: self.string_to_order_status(&r.get::<String, _>("status")),
            created_at: r.get("created_at"),
            expires_at: r.get("expires_at"),
            solver_account: r.get("solver_account"),
        }
    }

    fn row_to_trade(&self, r: sqlx::postgres::PgRow) -> Trade {
        Trade {
            trade_id: r.get("trade_id"),
            market_id: r.get("market_id"),
            condition_id: r.get("condition_id"),
            maker_order_id: r.get("maker_order_id"),
            taker_order_id: r.get("taker_order_id"),
            maker_account: r.get("maker_account"),
            taker_account: r.get("taker_account"),
            maker_side: self.string_to_order_side(&r.get::<String, _>("maker_side")),
            taker_side: self.string_to_order_side(&r.get::<String, _>("taker_side")),
            outcome: r.get::<i16, _>("outcome") as u8,
            price: r.get::<i64, _>("price") as u64,
            size: r.get::<i64, _>("size") as u128,
            trade_type: self.string_to_trade_type(&r.get::<String, _>("trade_type")),
            executed_at: r.get("executed_at"),
            settlement_status: self.string_to_settlement_status(&r.get::<String, _>("settlement_status")),
            settlement_tx_hash: r.get("settlement_tx_hash"),
        }
    }

    fn order_side_to_string(&self, side: &OrderSide) -> &'static str {
        match side {
            OrderSide::Buy => "Buy",
            OrderSide::Sell => "Sell",
        }
    }

    fn string_to_order_side(&self, s: &str) -> OrderSide {
        match s {
            "Buy" => OrderSide::Buy,
            "Sell" => OrderSide::Sell,
            _ => OrderSide::Buy,
        }
    }

    fn order_type_to_string(&self, order_type: &OrderType) -> &'static str {
        match order_type {
            OrderType::Limit => "Limit",
            OrderType::Market => "Market",
            OrderType::GTC => "GTC",
            OrderType::FOK => "FOK",
            OrderType::GTD => "GTD",
            OrderType::FAK => "FAK",
        }
    }

    fn string_to_order_type(&self, s: &str) -> OrderType {
        match s {
            "Limit" => OrderType::Limit,
            "Market" => OrderType::Market,
            "GTC" => OrderType::GTC,
            "FOK" => OrderType::FOK,
            "GTD" => OrderType::GTD,
            "FAK" => OrderType::FAK,
            _ => OrderType::Limit, // Default fallback
        }
    }

    fn order_status_to_string(&self, status: &OrderStatus) -> &'static str {
        match status {
            OrderStatus::Pending => "Pending",
            OrderStatus::PartiallyFilled => "PartiallyFilled",
            OrderStatus::Filled => "Filled",
            OrderStatus::Cancelled => "Cancelled",
            OrderStatus::Expired => "Expired",
            OrderStatus::Failed => "Failed",
        }
    }

    fn string_to_order_status(&self, s: &str) -> OrderStatus {
        match s {
            "Pending" => OrderStatus::Pending,
            "PartiallyFilled" => OrderStatus::PartiallyFilled,
            "Filled" => OrderStatus::Filled,
            "Cancelled" => OrderStatus::Cancelled,
            "Expired" => OrderStatus::Expired,
            "Failed" => OrderStatus::Failed,
            _ => OrderStatus::Pending,
        }
    }

    fn trade_type_to_string(&self, trade_type: &TradeType) -> &'static str {
        match trade_type {
            TradeType::DirectMatch => "DirectMatch",
            TradeType::Minting => "Minting",
            TradeType::Burning => "Burning",
        }
    }

    fn string_to_trade_type(&self, s: &str) -> TradeType {
        match s {
            "DirectMatch" => TradeType::DirectMatch,
            "Minting" => TradeType::Minting,
            "Burning" => TradeType::Burning,
            _ => TradeType::DirectMatch,
        }
    }

    fn settlement_status_to_string(&self, status: &SettlementStatus) -> &'static str {
        match status {
            SettlementStatus::Pending => "Pending",
            SettlementStatus::Settling => "Settling",
            SettlementStatus::Settled => "Settled",
            SettlementStatus::Failed => "Failed",
        }
    }

    fn string_to_settlement_status(&self, s: &str) -> SettlementStatus {
        match s {
            "Pending" => SettlementStatus::Pending,
            "Settling" => SettlementStatus::Settling,
            "Settled" => SettlementStatus::Settled,
            "Failed" => SettlementStatus::Failed,
            _ => SettlementStatus::Failed,
        }
    }
}