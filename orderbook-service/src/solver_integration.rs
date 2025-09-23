// Integration with NEAR Solver Contract for Polymarket-style workflow
// User -> Solver -> Orderbook -> Settlement via CTF

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::RwLock;
use anyhow::Result;
use serde_json::json;
use tracing::{info, warn, error};
use uuid::Uuid;
use chrono::Utc;

use crate::types::{Order, OrderSide, OrderType, OrderStatus, Trade, TradeType};
use crate::matching::MatchingEngine;
use crate::near_client::NearClient;

// NEAR contract call structures matching the solver contract
#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct SolverOrder {
    pub order_id: String,
    pub intent_id: String,
    pub user: String,
    pub market_id: String,
    pub condition_id: String,
    pub outcome: u8,
    pub side: SolverOrderSide,
    pub order_type: SolverOrderType,
    pub price: u64,
    pub amount: String, // U128 as string
    pub filled_amount: String,
    pub status: SolverOrderStatus,
    pub created_at: u64,
    pub expires_at: u64,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum SolverOrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum SolverOrderType {
    Market,
    Limit,
    GTC,    // Good-Till-Canceled
    FOK,    // Fill-or-Kill
    GTD,    // Good-Till-Date
    FAK,    // Fill-and-Kill
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub enum SolverOrderStatus {
    Pending,
    PartiallyFilled,
    Filled,
    Cancelled,
    Expired,
}

#[derive(serde::Deserialize, serde::Serialize, Clone)]
pub struct TradeExecutionRequest {
    pub trade_id: String,
    pub maker_order_id: String,
    pub taker_order_id: String,
    pub market_id: String,
    pub condition_id: String,
    pub outcome: u8,
    pub price: u64,
    pub amount: String, // U128 as string
    pub trade_type: SolverTradeType,
    pub maker: String,
    pub taker: String,
    pub executed_at: u64,
}

#[derive(serde::Deserialize, serde::Serialize, Clone, Debug)]
pub enum SolverTradeType {
    DirectMatch,
    Minting,
    Burning,
}

pub struct SolverIntegration {
    near_client: Arc<NearClient>,
    matching_engine: Arc<MatchingEngine>,
    solver_contract_id: String,
    // Map orderbook UUID -> solver string ID for settlement callbacks
    order_id_mapping: Arc<RwLock<HashMap<Uuid, String>>>,
}

impl SolverIntegration {
    pub fn new(
        near_client: Arc<NearClient>,
        matching_engine: Arc<MatchingEngine>,
        solver_contract_id: String,
    ) -> Self {
        Self {
            near_client,
            matching_engine,
            solver_contract_id,
            order_id_mapping: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Convert incoming solver order to orderbook order format
    pub async fn process_solver_order(&self, solver_order: SolverOrder) -> Result<Vec<Trade>> {
        info!("Processing solver order: {}", solver_order.order_id);

        // Look up the real condition ID for this market (don't trust solver's condition_id)
        let real_condition_id = match self.near_client.get_market_condition_id(&solver_order.market_id).await {
            Ok(Some(id)) => {
                info!("Using real condition ID for market {}: {}", solver_order.market_id, id);
                id
            }
            Ok(None) => {
                warn!("No condition ID found for market {}, using solver's condition_id", solver_order.market_id);
                solver_order.condition_id.clone()
            }
            Err(e) => {
                error!("Failed to look up condition ID for market {}: {}, using solver's condition_id", solver_order.market_id, e);
                solver_order.condition_id.clone()
            }
        };

        // Convert solver order format to orderbook order format
        let orderbook_order_id = Uuid::new_v4(); // Create new UUID for internal use

        // Store mapping from orderbook UUID to solver string ID for later settlement
        {
            let mut mapping = self.order_id_mapping.write().await;
            mapping.insert(orderbook_order_id, solver_order.order_id.clone());
        }

        info!("Mapped orderbook UUID {} to solver ID {}", orderbook_order_id, solver_order.order_id);
        let order = Order {
            order_id: orderbook_order_id,
            market_id: solver_order.market_id.clone(),
            condition_id: real_condition_id,
            user_account: solver_order.user.clone(),
            outcome: solver_order.outcome,
            side: match solver_order.side {
                SolverOrderSide::Buy => OrderSide::Buy,
                SolverOrderSide::Sell => OrderSide::Sell,
            },
            order_type: match solver_order.order_type {
                SolverOrderType::Market => OrderType::Market,
                SolverOrderType::Limit => OrderType::Limit,
                SolverOrderType::GTC => OrderType::GTC,
                SolverOrderType::FOK => OrderType::FOK,
                SolverOrderType::GTD => OrderType::GTD,
                SolverOrderType::FAK => OrderType::FAK,
            },
            price: solver_order.price,
            original_size: solver_order.amount.parse::<u128>()?,
            remaining_size: solver_order.amount.parse::<u128>()?,
            filled_size: solver_order.filled_amount.parse::<u128>()?,
            status: match solver_order.status {
                SolverOrderStatus::Pending => OrderStatus::Pending,
                SolverOrderStatus::PartiallyFilled => OrderStatus::PartiallyFilled,
                SolverOrderStatus::Filled => OrderStatus::Filled,
                SolverOrderStatus::Cancelled => OrderStatus::Cancelled,
                SolverOrderStatus::Expired => OrderStatus::Expired,
            },
            created_at: Utc::now(), // Use current time since we're processing now
            expires_at: if solver_order.expires_at > 0 {
                Some(Utc::now() + chrono::Duration::nanoseconds(solver_order.expires_at as i64))
            } else {
                None
            },
            solver_account: self.solver_contract_id.clone(),
        };

        // Mapping already stored above for settlement callbacks

        // VALIDATION STEP 1: Validate order parameters before submission
        if let Err(e) = self.validate_order_parameters(&order).await {
            error!("Order validation failed for {}: {}", order.order_id, e);
            return Err(e);
        }

        // VALIDATION STEP 2: Skip balance validation here to avoid double-reservation
        // Balance validation and reservation will be handled atomically by the matching engine
        info!("⏭️ Skipping solver-side balance validation to prevent double-reservation (handled by matching engine)");

        // Submit to matching engine
        info!("SOLVER: Multi-market processing - submitting order: {:?} {} {} @ {} for market {}",
            order.side, order.remaining_size,
            if order.outcome == 1 { "YES" } else { "NO" },
            order.price, order.market_id);

        let trades = self.matching_engine.submit_order(order).await?;

        info!("SOLVER: Order submitted, generated {} trades", trades.len());

        // For each trade generated, settle it via the solver contract
        for trade in &trades {
            if let Err(e) = self.settle_trade_via_solver(trade).await {
                error!("Failed to settle trade {} via solver: {}", trade.trade_id, e);
            }
        }

        info!("Processed solver order {} -> {} trades", solver_order.order_id, trades.len());
        Ok(trades)
    }

    /// Send trade back to solver contract for settlement via CTF
    async fn settle_trade_via_solver(&self, trade: &Trade) -> Result<()> {
        // Look up the original solver order IDs using our mapping
        info!("Looking up solver IDs for maker: {}, taker: {}", trade.maker_order_id, trade.taker_order_id);
        let (maker_solver_id, taker_solver_id) = {
            let mapping = self.order_id_mapping.read().await;
            info!("Current order mapping has {} entries", mapping.len());

            let maker_solver_id = mapping.get(&trade.maker_order_id)
                .ok_or_else(|| anyhow::anyhow!("No solver ID found for maker order {}", trade.maker_order_id))?
                .clone();
            let taker_solver_id = mapping.get(&trade.taker_order_id)
                .ok_or_else(|| anyhow::anyhow!("No solver ID found for taker order {}", trade.taker_order_id))?
                .clone();
            (maker_solver_id, taker_solver_id)
        };

        info!("Settling trade with solver IDs: maker={}, taker={}", maker_solver_id, taker_solver_id);
        info!("Trade details: trade_id={}, price={}, size={}, maker_account={}, taker_account={}",
              trade.trade_id, trade.price, trade.size, trade.maker_account, trade.taker_account);

        let trade_execution = TradeExecutionRequest {
            trade_id: trade.trade_id.to_string(),
            maker_order_id: maker_solver_id.clone(),
            taker_order_id: taker_solver_id.clone(),
            market_id: trade.market_id.clone(),
            condition_id: trade.condition_id.clone(),
            outcome: trade.outcome,
            price: trade.price,
            amount: trade.size.to_string(),
            trade_type: match trade.trade_type {
                TradeType::DirectMatch => SolverTradeType::DirectMatch,
                TradeType::Minting => SolverTradeType::Minting,
                TradeType::Burning => SolverTradeType::Burning,
            },
            maker: trade.maker_account.clone(),
            taker: trade.taker_account.clone(),
            executed_at: trade.executed_at.timestamp() as u64,
        };

        // Create orderbook signature (in production this would be cryptographically signed)
        let _orderbook_signature = format!("orderbook_v1_{}", trade.trade_id);

        // Update both maker and taker order fill status in solver contract
        info!(
            "Updating order fills for trade {} via solver contract: {} {} @ {} bps",
            trade.trade_id, trade.size, 
            if matches!(trade_execution.trade_type, SolverTradeType::DirectMatch) { "DIRECT" }
            else if matches!(trade_execution.trade_type, SolverTradeType::Minting) { "MINT" }
            else { "BURN" },
            trade.price
        );

        // Update maker order
        let maker_args = json!({
            "order_id": trade_execution.maker_order_id,
            "filled_amount": trade_execution.amount
        });

        info!("Calling update_order_fill for maker with args: {}", maker_args);
        let maker_tx_hash = match self.near_client
            .call_near_contract(
                &self.solver_contract_id,
                "update_order_fill",
                &maker_args.to_string(),
                "30000000000000", // 30 TGas for simple order update
                "0" // No deposit needed
            )
            .await {
                Ok(tx_hash) => {
                    info!("✅ Maker order {} update successful: {}", trade_execution.maker_order_id, tx_hash);
                    tx_hash
                }
                Err(e) if e.to_string().contains("Order not found") => {
                    info!("⚠️ Maker order {} no longer exists in solver (likely already completed), skipping update", trade_execution.maker_order_id);
                    "skipped_maker".to_string()
                }
                Err(e) if e.to_string().contains("panicked at") && e.to_string().contains("Order not found") => {
                    info!("⚠️ Maker order {} no longer exists in solver (contract panic), skipping update", trade_execution.maker_order_id);
                    "skipped_maker_panic".to_string()
                }
                Err(e) => {
                    error!("❌ Failed to update maker order {}: {}", trade_execution.maker_order_id, e);
                    // Don't fail the entire settlement for order update issues
                    warn!("Continuing settlement despite maker order update failure");
                    "failed_maker".to_string()
                }
            };

        // Update taker order
        let taker_args = json!({
            "order_id": trade_execution.taker_order_id,
            "filled_amount": trade_execution.amount
        });

        info!("Calling update_order_fill for taker with args: {}", taker_args);
        let taker_tx_hash = match self.near_client
            .call_near_contract(
                &self.solver_contract_id,
                "update_order_fill",
                &taker_args.to_string(),
                "30000000000000", // 30 TGas for simple order update
                "0" // No deposit needed
            )
            .await {
                Ok(tx_hash) => {
                    info!("✅ Taker order {} update successful: {}", trade_execution.taker_order_id, tx_hash);
                    tx_hash
                }
                Err(e) if e.to_string().contains("Order not found") => {
                    info!("⚠️ Taker order {} no longer exists in solver (likely already completed), skipping update", trade_execution.taker_order_id);
                    "skipped_taker".to_string()
                }
                Err(e) if e.to_string().contains("panicked at") && e.to_string().contains("Order not found") => {
                    info!("⚠️ Taker order {} no longer exists in solver (contract panic), skipping update", trade_execution.taker_order_id);
                    "skipped_taker_panic".to_string()
                }
                Err(e) => {
                    error!("❌ Failed to update taker order {}: {}", trade_execution.taker_order_id, e);
                    // Don't fail the entire settlement for order update issues
                    warn!("Continuing settlement despite taker order update failure");
                    "failed_taker".to_string()
                }
            };

        let tx_hash = format!("maker:{},taker:{}", maker_tx_hash, taker_tx_hash);

        info!("Trade {} settlement submitted to solver: {}", trade.trade_id, tx_hash);
        Ok(())
    }


    /// Process multiple solver orders in batch (useful for intent batching)
    pub async fn process_solver_orders_batch(&self, orders: Vec<SolverOrder>) -> Result<Vec<Trade>> {
        let mut all_trades = Vec::new();

        for order in orders {
            match self.process_solver_order(order).await {
                Ok(trades) => all_trades.extend(trades),
                Err(e) => {
                    error!("Failed to process solver order: {}", e);
                    // Continue processing other orders even if one fails
                }
            }
        }

        info!("Batch processed {} solver orders -> {} total trades", 
              all_trades.len(), all_trades.len());

        Ok(all_trades)
    }

    /// Validate order parameters before submission (like Polymarket's client-side validation)
    async fn validate_order_parameters(&self, order: &Order) -> Result<()> {
        // 1. Validate market exists
        if order.market_id.is_empty() {
            return Err(anyhow::anyhow!("Market ID cannot be empty"));
        }

        // 2. Validate user account
        if order.user_account.is_empty() {
            return Err(anyhow::anyhow!("User account cannot be empty"));
        }

        // 3. Validate outcome (binary market: 0 or 1)
        if order.outcome > 1 {
            return Err(anyhow::anyhow!("Outcome must be 0 (NO) or 1 (YES) for binary markets"));
        }

        // 4. Validate order size
        if order.original_size == 0 {
            return Err(anyhow::anyhow!("Order size must be greater than 0"));
        }

        // 5. Validate price based on order type (new format: 0-100000, where 100000 = $1.00)
        match order.order_type {
            OrderType::Limit | OrderType::GTC | OrderType::GTD => {
                if order.price == 0 {
                    return Err(anyhow::anyhow!("Limit/GTC/GTD orders cannot have zero price - use Market order instead"));
                }
                if order.price > 100000 {
                    return Err(anyhow::anyhow!("Price cannot exceed 100000 ($1.00)"));
                }
            }
            OrderType::FOK | OrderType::FAK => {
                if order.price == 0 {
                    return Err(anyhow::anyhow!("FOK/FAK orders must specify a price"));
                }
                if order.price > 100000 {
                    return Err(anyhow::anyhow!("Price cannot exceed 100000 ($1.00)"));
                }
            }
            OrderType::Market => {
                if order.price != 0 {
                    return Err(anyhow::anyhow!("Market orders should not specify a price (should be 0)"));
                }
            }
        }

        // 6. Validate condition ID exists for the market
        match self.near_client.get_market_condition_id(&order.market_id).await {
            Ok(Some(_)) => {
                info!("✅ Market {} has valid condition ID", order.market_id);
            }
            Ok(None) => {
                return Err(anyhow::anyhow!("Market {} does not have a registered condition ID", order.market_id));
            }
            Err(e) => {
                return Err(anyhow::anyhow!("Failed to validate market {}: {}", order.market_id, e));
            }
        }

        info!("✅ Order parameters validated successfully for order {}", order.order_id);
        Ok(())
    }

    /// Validate and reserve balance before order submission
    async fn validate_and_reserve_balance(&self, order: &Order) -> Result<()> {
        let collateral_manager = self.matching_engine.get_collateral_manager();

        // Check and reserve balance in one atomic operation
        let balance_sufficient = collateral_manager
            .check_and_reserve_balance(order)
            .await
            .map_err(|e| anyhow::anyhow!("Balance check and reservation failed: {}", e))?;

        if !balance_sufficient {
            return Err(anyhow::anyhow!(
                "Insufficient balance for order: user {} needs {} tokens for {} {} order in market {}",
                order.user_account,
                order.original_size,
                if order.outcome == 1 { "YES" } else { "NO" },
                match order.side {
                    OrderSide::Buy => "buy",
                    OrderSide::Sell => "sell"
                },
                order.market_id
            ));
        }

        info!("✅ Balance validated and reserved for order {} (user: {}, amount: {})",
            order.order_id, order.user_account, order.original_size);

        Ok(())
    }

    /// Get orderbook snapshot for a specific market (used by solver for price discovery)
    pub async fn get_market_liquidity(&self, market_id: &str, outcome: u8) -> Result<serde_json::Value> {
        let snapshot = self.matching_engine
            .get_orderbook_snapshot(market_id, outcome)
            .await?;

        if let Some(snapshot) = snapshot {
            Ok(json!({
                "market_id": snapshot.market_id,
                "outcome": snapshot.outcome,
                "bids": snapshot.bids.iter().map(|level| json!({
                    "price": level.price.to_string(),
                    "size": level.size.to_string(),
                    "orders": level.order_count
                })).collect::<Vec<_>>(),
                "asks": snapshot.asks.iter().map(|level| json!({
                    "price": level.price.to_string(),
                    "size": level.size.to_string(),
                    "orders": level.order_count
                })).collect::<Vec<_>>(),
                "last_price": snapshot.last_trade_price,
                "timestamp": snapshot.timestamp
            }))
        } else {
            Ok(json!({
                "market_id": market_id,
                "outcome": outcome,
                "bids": [],
                "asks": [],
                "last_price": null,
                "timestamp": Utc::now()
            }))
        }
    }

    /// Get current market price for solver's price calculations
    pub async fn get_market_price(&self, market_id: &str, outcome: u8) -> Result<Option<u64>> {
        let price_info = self.matching_engine
            .get_market_price(market_id, outcome)
            .await?;

        if let Some(price_info) = price_info {
            // Return mid-price if available, otherwise best bid or ask
            Ok(price_info.mid.or(price_info.bid).or(price_info.ask))
        } else {
            Ok(None)
        }
    }
}

/// HTTP API endpoints for solver integration
pub mod api {
    use super::*;
    use axum::{
        extract::{Path, State},
        http::StatusCode,
        response::IntoResponse,
        Json,
    };
    use crate::AppState;

    // Submit order from solver contract
    pub async fn submit_solver_order(
        State(app_state): State<AppState>,
        Json(order): Json<SolverOrder>,
    ) -> impl IntoResponse {
        match app_state.solver_integration.process_solver_order(order).await {
            Ok(trades) => (
                StatusCode::OK,
                Json(json!({
                    "success": true,
                    "trades_generated": trades.len(),
                    "trades": trades.iter().map(|t| json!({
                        "trade_id": t.trade_id,
                        "price": t.price,
                        "size": t.size,
                        "maker": t.maker_account,
                        "taker": t.taker_account
                    })).collect::<Vec<_>>()
                }))
            ).into_response(),
            Err(e) => {
                error!("Failed to process solver order: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "success": false,
                        "error": e.to_string()
                    }))
                ).into_response()
            }
        }
    }

    // Get market liquidity for solver
    pub async fn get_market_liquidity(
        State(app_state): State<AppState>,
        Path((market_id, outcome)): Path<(String, u8)>,
    ) -> impl IntoResponse {
        match app_state.solver_integration.get_market_liquidity(&market_id, outcome).await {
            Ok(liquidity) => (StatusCode::OK, Json(liquidity)).into_response(),
            Err(e) => {
                error!("Failed to get market liquidity: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": e.to_string()
                    }))
                ).into_response()
            }
        }
    }

    // Get current market price for solver
    pub async fn get_market_price(
        State(app_state): State<AppState>,
        Path((market_id, outcome)): Path<(String, u8)>,
    ) -> impl IntoResponse {
        match app_state.solver_integration.get_market_price(&market_id, outcome).await {
            Ok(price) => (
                StatusCode::OK,
                Json(json!({
                    "market_id": market_id,
                    "outcome": outcome,
                    "price": price,
                    "timestamp": Utc::now()
                }))
            ).into_response(),
            Err(e) => {
                error!("Failed to get market price: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({
                        "error": e.to_string()
                    }))
                ).into_response()
            }
        }
    }}
