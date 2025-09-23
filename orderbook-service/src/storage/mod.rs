// Database layer for persistent storage

use uuid::Uuid;
use anyhow::{anyhow, Result};
use chrono::Utc;
use std::collections::HashMap;
use std::sync::RwLock;

use crate::types::{Order, Trade, SettlementStatus, CollateralBalance, CollateralReservation};

// Simplified PostgreSQL implementation (runtime queries)
pub mod simple_postgres;
pub use simple_postgres::SimplePostgresDatabase;

// Database factory and trait
pub mod factory;
pub use factory::{DatabaseTrait, create_database, create_test_database};

// Simple in-memory database for testing
pub struct Database {
    orders: RwLock<HashMap<Uuid, Order>>,
    trades: RwLock<HashMap<Uuid, Trade>>,
    // Polymarket-style collateral storage
    collateral_balances: RwLock<HashMap<String, CollateralBalance>>, // key: "account:market"
    collateral_reservations: RwLock<HashMap<Uuid, CollateralReservation>>, // key: order_id
}

impl Database {
    pub async fn new() -> Result<Self> {
        Ok(Self {
            orders: RwLock::new(HashMap::new()),
            trades: RwLock::new(HashMap::new()),
            collateral_balances: RwLock::new(HashMap::new()),
            collateral_reservations: RwLock::new(HashMap::new()),
        })
    }

    pub async fn new_test() -> Result<Self> {
        Self::new().await
    }

    pub async fn insert_order(&self, order: &Order) -> Result<()> {
        let mut orders = self.orders.write()
            .map_err(|e| anyhow!("Failed to acquire write lock on orders: {}", e))?;
        orders.insert(order.order_id, order.clone());
        Ok(())
    }

    pub async fn update_order(&self, order: &Order) -> Result<()> {
        let mut orders = self.orders.write()
            .map_err(|e| anyhow!("Failed to acquire write lock on orders: {}", e))?;
        orders.insert(order.order_id, order.clone());
        Ok(())
    }

    pub async fn get_order(&self, order_id: Uuid) -> Result<Option<Order>> {
        let orders = self.orders.read()
            .map_err(|e| anyhow!("Failed to acquire read lock on orders: {}", e))?;
        Ok(orders.get(&order_id).cloned())
    }

    pub async fn get_active_orders(&self) -> Result<Vec<Order>> {
        let orders = self.orders.read()
            .map_err(|e| anyhow!("Failed to acquire read lock on orders: {}", e))?;
        Ok(orders.values()
            .filter(|o| matches!(o.status, crate::types::OrderStatus::Pending | crate::types::OrderStatus::PartiallyFilled))
            .cloned()
            .collect())
    }

    pub async fn get_expired_orders(&self) -> Result<Vec<Order>> {
        let orders = self.orders.read()
            .map_err(|e| anyhow!("Failed to acquire read lock on orders: {}", e))?;
        let now = Utc::now();
        
        Ok(orders.values()
            .filter(|o| {
                if let Some(expires_at) = o.expires_at {
                    expires_at < now && matches!(o.status, crate::types::OrderStatus::Pending | crate::types::OrderStatus::PartiallyFilled)
                } else {
                    false
                }
            })
            .cloned()
            .collect())
    }

    pub async fn insert_trade(&self, trade: &Trade) -> Result<()> {
        let mut trades = self.trades.write()
            .map_err(|e| anyhow!("Failed to acquire write lock on trades: {}", e))?;
        trades.insert(trade.trade_id, trade.clone());
        Ok(())
    }

    pub async fn update_trade_settlement_status(
        &self,
        trade_id: Uuid,
        status: SettlementStatus,
        tx_hash: Option<String>,
    ) -> Result<()> {
        let mut trades = self.trades.write()
            .map_err(|e| anyhow!("Failed to acquire write lock on trades: {}", e))?;
        if let Some(trade) = trades.get_mut(&trade_id) {
            trade.settlement_status = status;
            trade.settlement_tx_hash = tx_hash;
        }
        Ok(())
    }

    pub async fn get_failed_trades(&self) -> Result<Vec<Trade>> {
        let trades = self.trades.read()
            .map_err(|e| anyhow!("Failed to acquire read lock on trades: {}", e))?;
        Ok(trades.values()
            .filter(|t| matches!(t.settlement_status, SettlementStatus::Failed))
            .cloned()
            .collect())
    }

    // Test-only methods
    pub async fn count_settled_trades(&self) -> Result<usize> {
        let trades = self.trades.read()
            .map_err(|e| anyhow!("Failed to acquire read lock on trades: {}", e))?;
        Ok(trades.values()
            .filter(|t| matches!(t.settlement_status, SettlementStatus::Settled))
            .count())
    }

    pub async fn count_failed_trades(&self) -> Result<usize> {
        let trades = self.trades.read()
            .map_err(|e| anyhow!("Failed to acquire read lock on trades: {}", e))?;
        Ok(trades.values()
            .filter(|t| matches!(t.settlement_status, SettlementStatus::Failed))
            .count())
    }

    pub async fn count_pending_trades(&self) -> Result<usize> {
        let trades = self.trades.read()
            .map_err(|e| anyhow!("Failed to acquire read lock on trades: {}", e))?;
        Ok(trades.values()
            .filter(|t| matches!(t.settlement_status, SettlementStatus::Pending))
            .count())
    }

    pub async fn get_trades_for_market(&self, market_id: &str) -> Result<Vec<Trade>> {
        let trades = self.trades.read()
            .map_err(|e| anyhow!("Failed to acquire read lock on trades: {}", e))?;
        Ok(trades.values()
            .filter(|t| t.market_id == market_id)
            .cloned()
            .collect())
    }

    pub async fn get_settled_trades_for_condition(&self, condition_id: &str) -> Result<Vec<Trade>> {
        let trades = self.trades.read()
            .map_err(|e| anyhow!("Failed to acquire read lock on trades: {}", e))?;
        Ok(trades.values()
            .filter(|t| t.condition_id == condition_id && matches!(t.settlement_status, SettlementStatus::Settled))
            .cloned()
            .collect())
    }

    pub async fn get_trade_settlement_status(&self, trade_id: Uuid) -> Result<SettlementStatus> {
        let trades = self.trades.read()
            .map_err(|e| anyhow!("Failed to acquire read lock on trades: {}", e))?;
        Ok(trades.get(&trade_id)
            .map(|t| t.settlement_status.clone())
            .unwrap_or(SettlementStatus::Failed))
    }

    // ================================
    // POLYMARKET-STYLE COLLATERAL DATABASE METHODS
    // ================================

    pub async fn get_collateral_balance(&self, account_id: &str, market_id: &str) -> Result<Option<CollateralBalance>> {
        let balances = self.collateral_balances.read()
            .map_err(|e| anyhow!("Failed to acquire read lock on collateral balances: {}", e))?;
        let key = format!("{}:{}", account_id, market_id);
        Ok(balances.get(&key).cloned())
    }

    pub async fn update_collateral_balance(&self, balance: &CollateralBalance) -> Result<()> {
        let mut balances = self.collateral_balances.write()
            .map_err(|e| anyhow!("Failed to acquire write lock on collateral balances: {}", e))?;
        let key = format!("{}:{}", balance.account_id, balance.market_id);
        balances.insert(key, balance.clone());
        Ok(())
    }

    pub async fn store_collateral_reservation(&self, reservation: &CollateralReservation) -> Result<()> {
        let mut reservations = self.collateral_reservations.write()
            .map_err(|e| anyhow!("Failed to acquire write lock on reservations: {}", e))?;
        reservations.insert(reservation.order_id, reservation.clone());
        Ok(())
    }

    pub async fn get_collateral_reservation(&self, order_id: Uuid) -> Result<Option<CollateralReservation>> {
        let reservations = self.collateral_reservations.read()
            .map_err(|e| anyhow!("Failed to acquire read lock on reservations: {}", e))?;
        Ok(reservations.get(&order_id).cloned())
    }

    pub async fn remove_collateral_reservation(&self, order_id: Uuid) -> Result<()> {
        let mut reservations = self.collateral_reservations.write()
            .map_err(|e| anyhow!("Failed to acquire write lock on reservations: {}", e))?;
        reservations.remove(&order_id);
        Ok(())
    }
}