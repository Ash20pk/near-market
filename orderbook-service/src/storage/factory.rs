// Database factory for switching between in-memory and PostgreSQL implementations
// Preserves existing functionality while adding PostgreSQL support

use anyhow::Result;
use std::sync::Arc;
use tracing::{info, error, warn};

use super::{Database, SimplePostgresDatabase};
use crate::types::{Order, Trade, SettlementStatus, CollateralBalance, CollateralReservation, OrderbookSnapshot, MarketPrice};
use uuid::Uuid;

pub enum DatabaseType {
    InMemory,
    PostgreSQL,
}

// Trait that both implementations will satisfy
#[async_trait::async_trait]
pub trait DatabaseTrait: Send + Sync {
    // Order operations
    async fn insert_order(&self, order: &Order) -> Result<()>;
    async fn update_order(&self, order: &Order) -> Result<()>;
    async fn get_order(&self, order_id: Uuid) -> Result<Option<Order>>;
    async fn get_active_orders(&self) -> Result<Vec<Order>>;
    async fn get_expired_orders(&self) -> Result<Vec<Order>>;

    // Orderbook queries (enhanced for PostgreSQL)
    async fn get_orderbook_snapshot(&self, market_id: &str, outcome: u8) -> Result<Option<OrderbookSnapshot>>;
    async fn get_market_price(&self, market_id: &str, outcome: u8) -> Result<Option<MarketPrice>>;

    // Trade operations
    async fn insert_trade(&self, trade: &Trade) -> Result<()>;
    async fn update_trade_settlement_status(&self, trade_id: Uuid, status: SettlementStatus, tx_hash: Option<String>) -> Result<()>;
    async fn get_failed_trades(&self) -> Result<Vec<Trade>>;

    // Test methods
    async fn count_settled_trades(&self) -> Result<usize>;
    async fn count_failed_trades(&self) -> Result<usize>;
    async fn count_pending_trades(&self) -> Result<usize>;
    async fn get_trades_for_market(&self, market_id: &str) -> Result<Vec<Trade>>;
    async fn get_settled_trades_for_condition(&self, condition_id: &str) -> Result<Vec<Trade>>;
    async fn get_trade_settlement_status(&self, trade_id: Uuid) -> Result<SettlementStatus>;

    // Collateral operations
    async fn get_collateral_balance(&self, account_id: &str, market_id: &str) -> Result<Option<CollateralBalance>>;
    async fn update_collateral_balance(&self, balance: &CollateralBalance) -> Result<()>;
    async fn store_collateral_reservation(&self, reservation: &CollateralReservation) -> Result<()>;
    async fn get_collateral_reservation(&self, order_id: Uuid) -> Result<Option<CollateralReservation>>;
    async fn remove_collateral_reservation(&self, order_id: Uuid) -> Result<()>;
}

// Implement trait for in-memory Database
#[async_trait::async_trait]
impl DatabaseTrait for Database {
    async fn insert_order(&self, order: &Order) -> Result<()> {
        self.insert_order(order).await
    }

    async fn update_order(&self, order: &Order) -> Result<()> {
        self.update_order(order).await
    }

    async fn get_order(&self, order_id: Uuid) -> Result<Option<Order>> {
        self.get_order(order_id).await
    }

    async fn get_active_orders(&self) -> Result<Vec<Order>> {
        self.get_active_orders().await
    }

    async fn get_expired_orders(&self) -> Result<Vec<Order>> {
        self.get_expired_orders().await
    }

    // For in-memory, implement basic orderbook snapshot from active orders
    async fn get_orderbook_snapshot(&self, market_id: &str, outcome: u8) -> Result<Option<OrderbookSnapshot>> {
        let orders = self.get_active_orders().await?;

        // Filter orders for this market and outcome
        let relevant_orders: Vec<Order> = orders.into_iter()
            .filter(|o| o.market_id == market_id && o.outcome == outcome)
            .collect();

        if relevant_orders.is_empty() {
            return Ok(None);
        }

        // Build price levels manually for in-memory implementation
        let mut bids = std::collections::BTreeMap::new();
        let mut asks = std::collections::BTreeMap::new();

        for order in relevant_orders {
            match order.side {
                crate::types::OrderSide::Buy => {
                    let entry = bids.entry(order.price).or_insert((0u128, 0u32));
                    entry.0 += order.remaining_size;
                    entry.1 += 1;
                }
                crate::types::OrderSide::Sell => {
                    let entry = asks.entry(order.price).or_insert((0u128, 0u32));
                    entry.0 += order.remaining_size;
                    entry.1 += 1;
                }
            }
        }

        let bid_levels: Vec<crate::types::PriceLevel> = bids.into_iter()
            .rev() // Highest price first for bids
            .map(|(price, (size, count))| crate::types::PriceLevel { price, size, order_count: count })
            .collect();

        let ask_levels: Vec<crate::types::PriceLevel> = asks.into_iter()
            .map(|(price, (size, count))| crate::types::PriceLevel { price, size, order_count: count })
            .collect();

        Ok(Some(crate::types::OrderbookSnapshot {
            market_id: market_id.to_string(),
            outcome,
            bids: bid_levels,
            asks: ask_levels,
            last_trade_price: None, // Not tracked in in-memory
            timestamp: chrono::Utc::now(),
        }))
    }

    async fn get_market_price(&self, market_id: &str, outcome: u8) -> Result<Option<MarketPrice>> {
        // Simple implementation for in-memory - could be enhanced
        let snapshot = self.get_orderbook_snapshot(market_id, outcome).await?;

        match snapshot {
            Some(s) => Ok(Some(crate::types::MarketPrice {
                market_id: market_id.to_string(),
                outcome,
                bid: s.bids.first().map(|b| b.price),
                ask: s.asks.first().map(|a| a.price),
                mid: None, // Could calculate if both bid and ask exist
                last: s.last_trade_price,
                timestamp: chrono::Utc::now(),
            })),
            None => Ok(None),
        }
    }

    async fn insert_trade(&self, trade: &Trade) -> Result<()> {
        self.insert_trade(trade).await
    }

    async fn update_trade_settlement_status(&self, trade_id: Uuid, status: SettlementStatus, tx_hash: Option<String>) -> Result<()> {
        self.update_trade_settlement_status(trade_id, status, tx_hash).await
    }

    async fn get_failed_trades(&self) -> Result<Vec<Trade>> {
        self.get_failed_trades().await
    }

    async fn count_settled_trades(&self) -> Result<usize> {
        self.count_settled_trades().await
    }

    async fn count_failed_trades(&self) -> Result<usize> {
        self.count_failed_trades().await
    }

    async fn count_pending_trades(&self) -> Result<usize> {
        self.count_pending_trades().await
    }

    async fn get_trades_for_market(&self, market_id: &str) -> Result<Vec<Trade>> {
        self.get_trades_for_market(market_id).await
    }

    async fn get_settled_trades_for_condition(&self, condition_id: &str) -> Result<Vec<Trade>> {
        self.get_settled_trades_for_condition(condition_id).await
    }

    async fn get_trade_settlement_status(&self, trade_id: Uuid) -> Result<SettlementStatus> {
        self.get_trade_settlement_status(trade_id).await
    }

    async fn get_collateral_balance(&self, account_id: &str, market_id: &str) -> Result<Option<CollateralBalance>> {
        self.get_collateral_balance(account_id, market_id).await
    }

    async fn update_collateral_balance(&self, balance: &CollateralBalance) -> Result<()> {
        self.update_collateral_balance(balance).await
    }

    async fn store_collateral_reservation(&self, reservation: &CollateralReservation) -> Result<()> {
        self.store_collateral_reservation(reservation).await
    }

    async fn get_collateral_reservation(&self, order_id: Uuid) -> Result<Option<CollateralReservation>> {
        self.get_collateral_reservation(order_id).await
    }

    async fn remove_collateral_reservation(&self, order_id: Uuid) -> Result<()> {
        self.remove_collateral_reservation(order_id).await
    }
}

// Implement trait for SimplePostgresDatabase
#[async_trait::async_trait]
impl DatabaseTrait for SimplePostgresDatabase {
    async fn insert_order(&self, order: &Order) -> Result<()> {
        self.insert_order(order).await
    }

    async fn update_order(&self, order: &Order) -> Result<()> {
        self.update_order(order).await
    }

    async fn get_order(&self, order_id: Uuid) -> Result<Option<Order>> {
        self.get_order(order_id).await
    }

    async fn get_active_orders(&self) -> Result<Vec<Order>> {
        self.get_active_orders().await
    }

    async fn get_expired_orders(&self) -> Result<Vec<Order>> {
        self.get_expired_orders().await
    }

    async fn get_orderbook_snapshot(&self, market_id: &str, outcome: u8) -> Result<Option<OrderbookSnapshot>> {
        self.get_orderbook_snapshot(market_id, outcome).await
    }

    async fn get_market_price(&self, market_id: &str, outcome: u8) -> Result<Option<MarketPrice>> {
        self.get_market_price(market_id, outcome).await
    }

    async fn insert_trade(&self, trade: &Trade) -> Result<()> {
        self.insert_trade(trade).await
    }

    async fn update_trade_settlement_status(&self, trade_id: Uuid, status: SettlementStatus, tx_hash: Option<String>) -> Result<()> {
        self.update_trade_settlement_status(trade_id, status, tx_hash).await
    }

    async fn get_failed_trades(&self) -> Result<Vec<Trade>> {
        self.get_failed_trades().await
    }

    async fn count_settled_trades(&self) -> Result<usize> {
        self.count_settled_trades().await
    }

    async fn count_failed_trades(&self) -> Result<usize> {
        self.count_failed_trades().await
    }

    async fn count_pending_trades(&self) -> Result<usize> {
        self.count_pending_trades().await
    }

    async fn get_trades_for_market(&self, market_id: &str) -> Result<Vec<Trade>> {
        self.get_trades_for_market(market_id).await
    }

    async fn get_settled_trades_for_condition(&self, condition_id: &str) -> Result<Vec<Trade>> {
        self.get_settled_trades_for_condition(condition_id).await
    }

    async fn get_trade_settlement_status(&self, trade_id: Uuid) -> Result<SettlementStatus> {
        self.get_trade_settlement_status(trade_id).await
    }

    async fn get_collateral_balance(&self, account_id: &str, market_id: &str) -> Result<Option<CollateralBalance>> {
        self.get_collateral_balance(account_id, market_id).await
    }

    async fn update_collateral_balance(&self, balance: &CollateralBalance) -> Result<()> {
        self.update_collateral_balance(balance).await
    }

    async fn store_collateral_reservation(&self, reservation: &CollateralReservation) -> Result<()> {
        self.store_collateral_reservation(reservation).await
    }

    async fn get_collateral_reservation(&self, order_id: Uuid) -> Result<Option<CollateralReservation>> {
        self.get_collateral_reservation(order_id).await
    }

    async fn remove_collateral_reservation(&self, order_id: Uuid) -> Result<()> {
        self.remove_collateral_reservation(order_id).await
    }
}

// Removed unused imports
use tokio::sync::OnceCell;

// Use async-safe OnceCell instead of lazy_static for better async handling
static DATABASE_INSTANCE: OnceCell<Arc<dyn DatabaseTrait>> = OnceCell::const_new();

pub async fn create_database() -> Result<Arc<dyn DatabaseTrait>> {
    // Use get_or_try_init to ensure only one database is created even under concurrent access
    let database = DATABASE_INSTANCE.get_or_try_init(|| async {
        let db_type = determine_database_type();

        let result: Result<Arc<dyn DatabaseTrait>> = match db_type {
            DatabaseType::PostgreSQL => {
                info!("🐘 Initializing PostgreSQL database connection...");
                info!("📋 DATABASE_URL found: {}", std::env::var("DATABASE_URL").unwrap_or_else(|_| "NOT_SET".to_string()));
                match SimplePostgresDatabase::new().await {
                    Ok(postgres_db) => {
                        info!("✅ PostgreSQL database connected successfully - USING POSTGRESQL");
                        Ok(Arc::new(postgres_db) as Arc<dyn DatabaseTrait>)
                    }
                    Err(e) => {
                        error!("❌ PostgreSQL connection failed: {}", e);
                        error!("🔍 Common causes:");
                        error!("   1. Database schema not created (run supabase-schema.sql)");
                        error!("   2. Network/firewall issues");
                        error!("   3. Invalid credentials");
                        error!("   4. SSL/TLS configuration issues");
                        warn!("🔄 Falling back to in-memory database");
                        let in_memory_db = Database::new().await?;
                        Ok(Arc::new(in_memory_db) as Arc<dyn DatabaseTrait>)
                    }
                }
            }
            DatabaseType::InMemory => {
                info!("💾 Using in-memory database");
                let in_memory_db = Database::new().await?;
                Ok(Arc::new(in_memory_db) as Arc<dyn DatabaseTrait>)
            }
        };
        result
    }).await?;

    info!("♻️ Using database connection ({})",
        if matches!(determine_database_type(), DatabaseType::PostgreSQL) { "PostgreSQL" } else { "In-Memory" });

    Ok(database.clone())
}

pub async fn create_test_database() -> Result<Arc<dyn DatabaseTrait>> {
    // For tests, prefer in-memory but allow PostgreSQL if configured
    let use_postgres = std::env::var("USE_POSTGRES_FOR_TESTS")
        .unwrap_or_else(|_| "false".to_string())
        .parse::<bool>()
        .unwrap_or(false);

    if use_postgres {
        info!("🧪 Using PostgreSQL for tests");
        match SimplePostgresDatabase::new_test().await {
            Ok(postgres_db) => Ok(Arc::new(postgres_db)),
            Err(e) => {
                warn!("Test PostgreSQL connection failed: {}, using in-memory", e);
                let in_memory_db = Database::new_test().await?;
                Ok(Arc::new(in_memory_db))
            }
        }
    } else {
        info!("🧪 Using in-memory database for tests");
        let in_memory_db = Database::new_test().await?;
        Ok(Arc::new(in_memory_db))
    }
}

fn determine_database_type() -> DatabaseType {
    // Check if DATABASE_URL is set for PostgreSQL
    if std::env::var("DATABASE_URL").is_ok() {
        DatabaseType::PostgreSQL
    } else {
        DatabaseType::InMemory
    }
}