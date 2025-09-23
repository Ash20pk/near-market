// High-performance order matching engine

use std::collections::BTreeMap;
use std::sync::Arc;
use tokio::sync::{RwLock, mpsc, broadcast};
use uuid::Uuid;
use anyhow::Result;
use tracing::{info, error, debug};
use chrono::Utc;

use crate::types::{Order, Trade, OrderStatus, OrderType, OrderSide, TradeType, WebSocketMessage};
use crate::storage::DatabaseTrait;
use crate::near_client::NearClient;
use crate::collateral::CollateralManager;

pub mod engine;
pub mod settlement;

use engine::OrderBook;
use settlement::SettlementManager;

pub struct MatchingEngine {
    // Market ID -> Outcome -> OrderBook
    orderbooks: Arc<RwLock<BTreeMap<String, BTreeMap<u8, OrderBook>>>>,
    database: Arc<dyn DatabaseTrait>,
    settlement_manager: Arc<SettlementManager>,
    collateral_manager: Arc<CollateralManager>,
    trade_sender: mpsc::UnboundedSender<Trade>,
    ws_broadcaster: broadcast::Sender<WebSocketMessage>,
}

impl MatchingEngine {
    pub async fn new(
        database: Arc<dyn DatabaseTrait>,
        near_client: Arc<NearClient>,
        ws_broadcaster: broadcast::Sender<WebSocketMessage>,
    ) -> Result<Self> {
        let settlement_manager = Arc::new(
            SettlementManager::new(database.clone(), near_client.clone()).await?
        );

        let collateral_manager = Arc::new(
            CollateralManager::new(database.clone(), near_client)
        );

        let (trade_sender, trade_receiver) = mpsc::unbounded_channel();

        // Start settlement worker
        let settlement_manager_clone = settlement_manager.clone();
        tokio::spawn(async move {
            if let Err(e) = settlement_manager_clone.run(trade_receiver).await {
                error!("Settlement manager crashed: {}", e);
            }
        });

        Ok(Self {
            orderbooks: Arc::new(RwLock::new(BTreeMap::new())),
            database,
            settlement_manager,
            collateral_manager,
            trade_sender,
            ws_broadcaster,
        })
    }

    pub async fn submit_order(&self, order: Order) -> Result<Vec<Trade>> {
        // Atomic transaction scope for order submission
        let transaction_result = self.execute_order_submission_transaction(order).await;

        match transaction_result {
            Ok((trades, order_stored)) => {
                // Broadcast successful order updates
                if !trades.is_empty() {
                    self.broadcast_order_updates(&trades).await;
                }

                info!("Order {} submitted successfully, generated {} trades",
                    order_stored.order_id, trades.len());

                Ok(trades)
            }
            Err(e) => {
                error!("Order submission failed: {}", e);
                Err(e)
            }
        }
    }

    /// Execute order submission as an atomic transaction
    async fn execute_order_submission_transaction(&self, order: Order) -> Result<(Vec<Trade>, Order)> {
        info!("Starting atomic order submission transaction for order {}", order.order_id);

        // Step 1: Acquire orderbook write lock FIRST to prevent race conditions
        let mut orderbooks = self.orderbooks.write().await;

        // Step 2: Check and reserve balance WITHIN the lock scope (Polymarket-style)
        let can_place_order = match self.collateral_manager.check_and_reserve_balance(&order).await {
            Ok(result) => {
                info!("Balance check for order {} completed: {}", order.order_id, result);
                result
            }
            Err(e) => {
                error!("Balance check failed for order {}: {}", order.order_id, e);
                return Err(e);
            }
        };

        if !can_place_order {
            error!("Insufficient balance to place order {}", order.order_id);
            return Err(anyhow::anyhow!("Insufficient balance to place order"));
        }

        // Get or create orderbook for this market+outcome
        let market_orderbooks = orderbooks
            .entry(order.market_id.clone())
            .or_insert_with(BTreeMap::new);

        // Step 3: Store order in database WITHIN the lock scope
        if let Err(e) = self.database.insert_order(&order).await {
            error!("Failed to store order {} in database: {}", order.order_id, e);
            return Err(e);
        }
        info!("Order {} stored in database successfully", order.order_id);

        // Create mutable copy of order to track fills
        let mut working_order = order.clone();

        // Step 4: Try regular orderbook matching FIRST (existing liquidity priority)
        let mut trades = Vec::new();

        if working_order.remaining_size > 0 {
            trades.extend(self.execute_regular_orderbook_matching(&mut working_order, market_orderbooks).await?);
        }

        // Step 5: Only try complementary matching if order still has remaining size after regular matching
        if working_order.remaining_size > 0 {
            match self.check_complementary_matches_mutable(&mut working_order, market_orderbooks).await {
                Ok(mint_trades) => {
                    if !mint_trades.is_empty() {
                        info!("✅ Complementary minting after no regular liquidity: {} trades", mint_trades.len());
                        trades.extend(mint_trades);
                    } else {
                        info!("📋 No complementary matches available, order added to orderbook");
                    }
                }
                Err(e) => {
                    info!("⚠️  Complementary matching skipped: {}", e);
                    // Don't fail the order - just continue without complementary matching
                }
            }
        }

        // Step 6: Add remaining order to orderbook if not fully filled
        if working_order.remaining_size > 0 {
            let orderbook = market_orderbooks
                .entry(working_order.outcome)
                .or_insert_with(OrderBook::new);

            orderbook.add_order(working_order.clone()).await?;
            info!("📋 Order {} added to orderbook with {} remaining", working_order.order_id, working_order.remaining_size);
        }

        // Step 4: Store trades atomically and send for settlement
        for trade in &trades {
            self.database.insert_trade(trade).await?;

            // Send for settlement (non-blocking)
            if let Err(e) = self.trade_sender.send(trade.clone()) {
                error!("Failed to send trade for settlement: {}", e);
                // Note: This is not fatal - settlement can be retried
            }

            // Broadcast trade execution via WebSocket (non-blocking)
            let ws_message = WebSocketMessage::TradeExecuted {
                trade: trade.clone(),
            };

            if let Err(e) = self.ws_broadcaster.send(ws_message) {
                error!("Failed to broadcast trade execution: {}", e);
            } else {
                info!("📡 Broadcasted trade execution: {} {} tokens @ {} bps",
                    trade.trade_id, trade.size, trade.price);
            }
        }

        // Final order state is already properly tracked in working_order
        // Just ensure the final state is in the database
        if !trades.is_empty() {
            self.database.update_order(&working_order).await?;
        }

        Ok((trades, working_order))
    }

    /// Check for complementary order matches (Polymarket-style unified orderbook)
    /// YES@60% + NO@40% = 100% should execute as mint operation
    async fn check_complementary_matches(
        &self,
        incoming_order: &Order,
        market_orderbooks: &mut BTreeMap<u8, OrderBook>,
    ) -> Result<Vec<Trade>> {
        // Only check complementary matches for limit-type orders in binary markets
        if !matches!(incoming_order.order_type, OrderType::Limit | OrderType::GTC | OrderType::GTD | OrderType::FOK | OrderType::FAK) {
            info!("⏭️  Skipping complementary match check: not a limit-type order");
            return Ok(Vec::new());
        }

        // Execute complementary matching as atomic transaction
        self.execute_complementary_match_transaction(incoming_order, market_orderbooks).await
    }

    /// Check for complementary order matches with mutable order (for proper state tracking)
    async fn check_complementary_matches_mutable(
        &self,
        incoming_order: &mut Order,
        market_orderbooks: &mut BTreeMap<u8, OrderBook>,
    ) -> Result<Vec<Trade>> {
        // Only check complementary matches for limit-type orders in binary markets
        if !matches!(incoming_order.order_type, OrderType::Limit | OrderType::GTC | OrderType::GTD | OrderType::FOK | OrderType::FAK) {
            info!("⏭️  Skipping complementary match check: not a limit-type order");
            return Ok(Vec::new());
        }

        // Skip complementary matching for market orders (price = 0)
        if incoming_order.price == 0 {
            info!("⏭️  Skipping complementary match check: market order (price=0)");
            return Ok(Vec::new());
        }

        // Execute complementary matching as atomic transaction with mutable order
        self.execute_complementary_match_transaction_mutable(incoming_order, market_orderbooks).await
    }

    /// Execute complementary matching as atomic transaction with proper validation
    async fn execute_complementary_match_transaction(
        &self,
        incoming_order: &Order,
        market_orderbooks: &mut BTreeMap<u8, OrderBook>,
    ) -> Result<Vec<Trade>> {
        // Determine complement outcome (YES=1, NO=0)
        let complement_outcome = if incoming_order.outcome == 1 { 0 } else { 1 };

        // Calculate complement price with explicit validation
        let complement_price = self.calculate_complement_price(incoming_order.price)?;

        info!("🔍 COMPLEMENTARY SEARCH: Incoming {}@{}% (outcome {}), looking for {}@{}% (outcome {})",
            incoming_order.remaining_size, incoming_order.price as f64 / 100.0, incoming_order.outcome,
            incoming_order.remaining_size, complement_price as f64 / 100.0, complement_outcome);

        // Get orderbook for complement outcome
        let complement_orderbook = match market_orderbooks.get_mut(&complement_outcome) {
            Some(orderbook) => {
                info!("✅ Found orderbook for complement outcome {}", complement_outcome);
                orderbook
            },
            None => {
                info!("❌ No orderbook exists for complement outcome {}", complement_outcome);
                return Ok(Vec::new()); // No orders on complement side
            }
        };

        // Look for matching orders on complement side with validation
        let matching_order_result = self.find_and_validate_complementary_order(
            complement_orderbook,
            complement_price,
            incoming_order.side.clone(),
            incoming_order.remaining_size
        ).await?;

        if let Some((mut maker_order, validated_trade_size)) = matching_order_result {
            info!("🎯 COMPLEMENTARY MATCH DETECTED!");
            info!("   Incoming: {}@{}% (outcome {})",
                incoming_order.original_size, incoming_order.price as f64 / 100.0, incoming_order.outcome);
            info!("   Matching: {}@{}% (outcome {})",
                maker_order.original_size, maker_order.price as f64 / 100.0, maker_order.outcome);
            info!("   Trade size: {} tokens", validated_trade_size);
            info!("   Total: {}% + {}% = {}% ✅",
                incoming_order.price as f64 / 100.0,
                maker_order.price as f64 / 100.0,
                (incoming_order.price + maker_order.price) as f64 / 100.0);

            // Store the order details before modification
            let maker_order_id = maker_order.order_id;
            let maker_price = maker_order.price;
            let maker_side = maker_order.side.clone();

            // Create complementary mint trade with validated size
            let trade = self.create_complementary_mint_trade_validated(
                incoming_order,
                &mut maker_order,
                validated_trade_size
            ).await?;

            // Atomically update the maker order in the database
            self.database.update_order(&maker_order).await?;

            // Remove the maker order from the complement orderbook if fully filled
            if maker_order.remaining_size == 0 {
                complement_orderbook.remove_specific_order(maker_order_id, maker_price, maker_side).await?;
            } else {
                // Update the order in the orderbook if partially filled
                complement_orderbook.update_order_size(maker_order_id, maker_order.remaining_size).await?;
            }

            return Ok(vec![trade]);
        }

        Ok(Vec::new())
    }

    /// Execute complementary matching as atomic transaction with mutable order
    async fn execute_complementary_match_transaction_mutable(
        &self,
        incoming_order: &mut Order,
        market_orderbooks: &mut BTreeMap<u8, OrderBook>,
    ) -> Result<Vec<Trade>> {
        // Determine complement outcome (YES=1, NO=0)
        let complement_outcome = if incoming_order.outcome == 1 { 0 } else { 1 };

        // Calculate complement price with explicit validation
        let complement_price = self.calculate_complement_price(incoming_order.price)?;

        info!("🔍 COMPLEMENTARY SEARCH: Incoming {}@{}% (outcome {}), looking for {}@{}% (outcome {})",
            incoming_order.remaining_size, incoming_order.price as f64 / 100.0, incoming_order.outcome,
            incoming_order.remaining_size, complement_price as f64 / 100.0, complement_outcome);

        // Get orderbook for complement outcome
        let complement_orderbook = match market_orderbooks.get_mut(&complement_outcome) {
            Some(orderbook) => {
                info!("✅ Found orderbook for complement outcome {}", complement_outcome);
                orderbook
            },
            None => {
                info!("❌ No orderbook exists for complement outcome {}", complement_outcome);
                return Ok(Vec::new()); // No orders on complement side
            }
        };

        // Look for matching orders on complement side with validation
        let matching_order_result = self.find_and_validate_complementary_order(
            complement_orderbook,
            complement_price,
            incoming_order.side.clone(),
            incoming_order.remaining_size
        ).await?;

        if let Some((mut maker_order, validated_trade_size)) = matching_order_result {
            info!("🎯 COMPLEMENTARY MATCH DETECTED!");
            info!("   Incoming: {}@{}% (outcome {})",
                incoming_order.original_size, incoming_order.price as f64 / 100.0, incoming_order.outcome);
            info!("   Matching: {}@{}% (outcome {})",
                maker_order.original_size, maker_order.price as f64 / 100.0, maker_order.outcome);
            info!("   Trade size: {} tokens", validated_trade_size);
            info!("   Total: {}% + {}% = {}% ✅",
                incoming_order.price as f64 / 100.0,
                maker_order.price as f64 / 100.0,
                (incoming_order.price + maker_order.price) as f64 / 100.0);

            // Store the order details before modification
            let maker_order_id = maker_order.order_id;
            let maker_price = maker_order.price;
            let maker_side = maker_order.side.clone();

            // Create complementary mint trade with validated size
            let trade = self.create_complementary_mint_trade_validated_mutable(
                incoming_order,
                &mut maker_order,
                validated_trade_size
            ).await?;

            // Atomically update the maker order in the database
            self.database.update_order(&maker_order).await?;

            // Remove the maker order from the complement orderbook if fully filled
            if maker_order.remaining_size == 0 {
                complement_orderbook.remove_specific_order(maker_order_id, maker_price, maker_side).await?;
            } else {
                // Update the order in the orderbook if partially filled
                complement_orderbook.update_order_size(maker_order_id, maker_order.remaining_size).await?;
            }

            return Ok(vec![trade]);
        }

        Ok(Vec::new())
    }

    /// Calculate complement price with validation
    fn calculate_complement_price(&self, price: u64) -> Result<u64> {
        if price > 99999 {
            return Err(anyhow::anyhow!("Invalid price: {} exceeds 99999 (0.99999)", price));
        }
        if price == 0 {
            // Zero price indicates market order - skip complementary matching
            return Err(anyhow::anyhow!("Market orders (price=0) skip complementary matching"));
        }
        Ok(100000 - price)  // Complement price: 60000 + 40000 = 100000 (full dollar)
    }

    /// Find and validate a complementary order that matches the required price and side
    async fn find_and_validate_complementary_order(
        &self,
        orderbook: &mut OrderBook,
        target_price: u64,
        incoming_side: OrderSide,
        incoming_size: u128,
    ) -> Result<Option<(Order, u128)>> {
        // For Polymarket-style complementary matching, we need SAME sides:
        // Buy YES@60% matches with Buy NO@40% (both are Buy orders)
        // Sell YES@60% matches with Sell NO@40% (both are Sell orders)
        let required_side = incoming_side.clone();

        // Look for orders at the exact complementary price with the same side
        if let Some(order) = orderbook.get_orders_by_price_and_side(target_price, required_side.clone()).await? {
            info!("🔍 Found complementary order candidate: {}@{} (side: {:?})",
                order.remaining_size, target_price, required_side);

            // Validate order is still active and not expired
            if !matches!(order.status, OrderStatus::Pending | OrderStatus::PartiallyFilled) {
                info!("❌ Complementary order has invalid status: {:?}", order.status);
                return Ok(None);
            }

            // Check expiration
            if let Some(expires_at) = order.expires_at {
                if Utc::now() > expires_at {
                    info!("❌ Complementary order has expired");
                    return Ok(None);
                }
            }

            // Calculate validated trade size
            let trade_size = std::cmp::min(incoming_size, order.remaining_size);
            if trade_size == 0 {
                info!("❌ No valid trade size for complementary match");
                return Ok(None);
            }

            info!("✅ Validated complementary order: trade size = {}", trade_size);
            return Ok(Some((order, trade_size)));
        }

        Ok(None)
    }

    /// Create a mint trade from two complementary orders with validated trade size
    async fn create_complementary_mint_trade_validated(
        &self,
        taker_order: &Order,
        maker_order: &mut Order,
        validated_trade_size: u128,
    ) -> Result<Trade> {
        use chrono::Utc;

        // Double-check trade size is valid
        if validated_trade_size == 0 {
            return Err(anyhow::anyhow!("Invalid trade size: cannot be zero"));
        }

        if validated_trade_size > taker_order.remaining_size ||
           validated_trade_size > maker_order.remaining_size {
            return Err(anyhow::anyhow!(
                "Invalid trade size: {} exceeds available capacity (taker: {}, maker: {})",
                validated_trade_size, taker_order.remaining_size, maker_order.remaining_size
            ));
        }

        // Verify complementary prices allow profitable minting (≤ $1.00)
        let total_price = taker_order.price + maker_order.price;
        if total_price > 100000 {
            return Err(anyhow::anyhow!(
                "Complementary prices exceed $1.00: {} + {} = {} (unprofitable minting)",
                taker_order.price as f64 / 100000.0, maker_order.price as f64 / 100000.0, total_price as f64 / 100000.0
            ));
        }

        // Calculate price improvement for traders
        let price_improvement = 100000 - total_price;
        if price_improvement > 0 {
            info!("💰 Price improvement: ${:.5} savings from minting at ${:.5} instead of $1.00",
                  price_improvement as f64 / 100000.0, total_price as f64 / 100000.0);
        }

        // Atomically update maker order state
        maker_order.remaining_size = maker_order.remaining_size.saturating_sub(validated_trade_size);
        maker_order.filled_size = maker_order.filled_size.saturating_add(validated_trade_size);
        maker_order.status = if maker_order.remaining_size == 0 {
            OrderStatus::Filled
        } else {
            OrderStatus::PartiallyFilled
        };

        // Create mint trade with validated parameters
        let trade = Trade {
            trade_id: Uuid::new_v4(),
            market_id: taker_order.market_id.clone(),
            condition_id: taker_order.condition_id.clone(),
            maker_order_id: maker_order.order_id,
            taker_order_id: taker_order.order_id,
            maker_account: maker_order.user_account.clone(),
            taker_account: taker_order.user_account.clone(),
            maker_side: maker_order.side.clone(),
            taker_side: taker_order.side.clone(),
            outcome: taker_order.outcome, // Trade recorded under taker's outcome
            price: taker_order.price, // Use taker's price for recording
            size: validated_trade_size,
            trade_type: TradeType::Minting, // This is a mint operation!
            executed_at: Utc::now(),
            settlement_status: crate::types::SettlementStatus::Pending,
            settlement_tx_hash: None,
        };

        info!("🔥 VALIDATED MINT TRADE CREATED: {} tokens @ complementary prices", validated_trade_size);
        info!("   YES user: {} gets {} YES tokens",
            if taker_order.outcome == 1 { &taker_order.user_account } else { &maker_order.user_account },
            validated_trade_size);
        info!("   NO user: {} gets {} NO tokens",
            if taker_order.outcome == 0 { &taker_order.user_account } else { &maker_order.user_account },
            validated_trade_size);

        Ok(trade)
    }

    /// Create a mint trade from two complementary orders with validated trade size (mutable taker)
    async fn create_complementary_mint_trade_validated_mutable(
        &self,
        taker_order: &mut Order,
        maker_order: &mut Order,
        validated_trade_size: u128,
    ) -> Result<Trade> {
        use chrono::Utc;

        // Double-check trade size is valid
        if validated_trade_size == 0 {
            return Err(anyhow::anyhow!("Invalid trade size: cannot be zero"));
        }

        if validated_trade_size > taker_order.remaining_size ||
           validated_trade_size > maker_order.remaining_size {
            return Err(anyhow::anyhow!(
                "Invalid trade size: {} exceeds available capacity (taker: {}, maker: {})",
                validated_trade_size, taker_order.remaining_size, maker_order.remaining_size
            ));
        }

        // Verify complementary prices allow profitable minting (≤ $1.00)
        let total_price = taker_order.price + maker_order.price;
        if total_price > 100000 {
            return Err(anyhow::anyhow!(
                "Complementary prices exceed $1.00: {} + {} = {} (unprofitable minting)",
                taker_order.price as f64 / 100000.0, maker_order.price as f64 / 100000.0, total_price as f64 / 100000.0
            ));
        }

        // Calculate price improvement for traders
        let price_improvement = 100000 - total_price;
        if price_improvement > 0 {
            info!("💰 Price improvement: ${:.5} savings from minting at ${:.5} instead of $1.00",
                  price_improvement as f64 / 100000.0, total_price as f64 / 100000.0);
        }

        // Atomically update both order states
        taker_order.remaining_size = taker_order.remaining_size.saturating_sub(validated_trade_size);
        taker_order.filled_size = taker_order.filled_size.saturating_add(validated_trade_size);
        taker_order.status = if taker_order.remaining_size == 0 {
            OrderStatus::Filled
        } else {
            OrderStatus::PartiallyFilled
        };

        maker_order.remaining_size = maker_order.remaining_size.saturating_sub(validated_trade_size);
        maker_order.filled_size = maker_order.filled_size.saturating_add(validated_trade_size);
        maker_order.status = if maker_order.remaining_size == 0 {
            OrderStatus::Filled
        } else {
            OrderStatus::PartiallyFilled
        };

        // Create mint trade with validated parameters
        let trade = Trade {
            trade_id: Uuid::new_v4(),
            market_id: taker_order.market_id.clone(),
            condition_id: taker_order.condition_id.clone(),
            maker_order_id: maker_order.order_id,
            taker_order_id: taker_order.order_id,
            maker_account: maker_order.user_account.clone(),
            taker_account: taker_order.user_account.clone(),
            maker_side: maker_order.side.clone(),
            taker_side: taker_order.side.clone(),
            outcome: taker_order.outcome, // Trade recorded under taker's outcome
            price: taker_order.price, // Use taker's price for recording
            size: validated_trade_size,
            trade_type: TradeType::Minting, // This is a mint operation!
            executed_at: Utc::now(),
            settlement_status: crate::types::SettlementStatus::Pending,
            settlement_tx_hash: None,
        };

        info!("🔥 VALIDATED MINT TRADE CREATED (MUTABLE): {} tokens @ complementary prices", validated_trade_size);
        info!("   YES user: {} gets {} YES tokens",
            if taker_order.outcome == 1 { &taker_order.user_account } else { &maker_order.user_account },
            validated_trade_size);
        info!("   NO user: {} gets {} NO tokens",
            if taker_order.outcome == 0 { &taker_order.user_account } else { &maker_order.user_account },
            validated_trade_size);

        Ok(trade)
    }

    pub async fn cancel_order(&self, order_id: Uuid, user_account: &str) -> Result<bool> {
        // Execute cancellation as atomic transaction to prevent race conditions
        self.execute_order_cancellation_transaction(order_id, user_account).await
    }

    /// Execute order cancellation as an atomic transaction
    async fn execute_order_cancellation_transaction(&self, order_id: Uuid, user_account: &str) -> Result<bool> {
        // Step 1: Acquire orderbook write lock FIRST
        let mut orderbooks = self.orderbooks.write().await;

        // Step 2: Retrieve and validate order WITHIN lock scope
        let mut order = self.database.get_order(order_id).await?
            .ok_or_else(|| anyhow::anyhow!("Order not found"))?;

        // Verify ownership
        if order.user_account != user_account {
            return Err(anyhow::anyhow!("Not authorized to cancel this order"));
        }

        // Can only cancel pending or partially filled orders
        if !matches!(order.status, OrderStatus::Pending | OrderStatus::PartiallyFilled) {
            return Err(anyhow::anyhow!("Cannot cancel order in status: {:?}", order.status));
        }

        // Step 3: Calculate balance to release based on CURRENT remaining size
        let balance_to_release = self.collateral_manager.calculate_required_balance(&order)?;

        // Step 4: Remove from orderbook atomically (within the same lock scope)
        let removal_successful = if let Some(market_orderbooks) = orderbooks.get_mut(&order.market_id) {
            if let Some(orderbook) = market_orderbooks.get_mut(&order.outcome) {
                orderbook.remove_order(order_id).await?;
                true
            } else {
                false
            }
        } else {
            false
        };

        if !removal_successful {
            return Err(anyhow::anyhow!("Order not found in orderbook - may have been filled"));
        }

        // Step 5: Update order status atomically
        order.status = OrderStatus::Cancelled;
        self.database.update_order(&order).await?;

        // Step 6: Release balance reservation back to user
        self.collateral_manager.release_market_balance(user_account, &order.market_id, balance_to_release).await?;

        info!("Order {} cancelled by {}, released {} balance",
            order_id, user_account, balance_to_release);

        Ok(true)
    }

    /// Broadcast order status updates for affected orders in trades
    async fn broadcast_order_updates(&self, trades: &[Trade]) {
        for trade in trades {
            // Broadcast update for maker order
            if let Ok(Some(maker_order)) = self.database.get_order(trade.maker_order_id).await {
                let ws_message = WebSocketMessage::OrderUpdate {
                    order_id: maker_order.order_id,
                    status: maker_order.status.clone(),
                    filled_size: maker_order.filled_size,
                };

                if let Err(e) = self.ws_broadcaster.send(ws_message) {
                    error!("Failed to broadcast maker order update: {}", e);
                } else {
                    info!("📡 Broadcasted maker order update: {} status={:?}, filled={}",
                        maker_order.order_id, maker_order.status, maker_order.filled_size);
                }
            }

            // Broadcast update for taker order
            if let Ok(Some(taker_order)) = self.database.get_order(trade.taker_order_id).await {
                let ws_message = WebSocketMessage::OrderUpdate {
                    order_id: taker_order.order_id,
                    status: taker_order.status.clone(),
                    filled_size: taker_order.filled_size,
                };

                if let Err(e) = self.ws_broadcaster.send(ws_message) {
                    error!("Failed to broadcast taker order update: {}", e);
                } else {
                    info!("📡 Broadcasted taker order update: {} status={:?}, filled={}",
                        taker_order.order_id, taker_order.status, taker_order.filled_size);
                }
            }
        }
    }

    pub async fn get_orderbook_snapshot(
        &self,
        market_id: &str,
        outcome: u8,
    ) -> Result<Option<crate::types::OrderbookSnapshot>> {
        // Try to get from database first (PostgreSQL will have real-time persistent data)
        if let Ok(Some(snapshot)) = self.database.get_orderbook_snapshot(market_id, outcome).await {
            info!("📊 Retrieved orderbook snapshot from database: {} bids, {} asks",
                snapshot.bids.len(), snapshot.asks.len());
            return Ok(Some(snapshot));
        }

        // Fallback to in-memory orderbooks
        let orderbooks = self.orderbooks.read().await;

        if let Some(market_orderbooks) = orderbooks.get(market_id) {
            if let Some(orderbook) = market_orderbooks.get(&outcome) {
                info!("📊 Retrieved orderbook snapshot from memory");
                return Ok(Some(orderbook.get_snapshot(market_id, outcome).await?));
            }
        }

        info!("📊 No orderbook data found for market {} outcome {}", market_id, outcome);
        Ok(None)
    }

    pub async fn get_market_price(
        &self,
        market_id: &str,
        outcome: u8,
    ) -> Result<Option<crate::types::MarketPrice>> {
        // Try to get from database first (PostgreSQL will have accurate market stats)
        if let Ok(Some(price)) = self.database.get_market_price(market_id, outcome).await {
            info!("💰 Retrieved market price from database: bid={:?}, ask={:?}",
                price.bid.map(|b| b as f64 / 100.0),
                price.ask.map(|a| a as f64 / 100.0));
            return Ok(Some(price));
        }

        // Fallback to in-memory orderbooks
        let orderbooks = self.orderbooks.read().await;

        if let Some(market_orderbooks) = orderbooks.get(market_id) {
            if let Some(orderbook) = market_orderbooks.get(&outcome) {
                info!("💰 Retrieved market price from memory");
                return Ok(Some(orderbook.get_market_price(market_id, outcome).await?));
            }
        }

        info!("💰 No market price data found for market {} outcome {}", market_id, outcome);
        Ok(None)
    }

    pub async fn run(&self) -> Result<()> {
        info!("Matching engine started");

        // Restore orderbooks from database on startup
        self.restore_orderbooks().await?;

        // Main event loop - could add periodic tasks here
        loop {
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
            
            // Periodic tasks:
            // 1. Expire old orders
            // 2. Cleanup empty price levels
            // 3. Update market prices
            // 4. Health checks
            self.expire_orders().await?;
            self.cleanup_empty_price_levels().await?;
        }
    }

    async fn restore_orderbooks(&self) -> Result<()> {
        info!("Restoring orderbooks from database...");
        
        let active_orders = self.database.get_active_orders().await?;
        let mut orderbooks = self.orderbooks.write().await;

        for order in active_orders {
            let market_orderbooks = orderbooks
                .entry(order.market_id.clone())
                .or_insert_with(BTreeMap::new);
            
            let orderbook = market_orderbooks
                .entry(order.outcome)
                .or_insert_with(OrderBook::new);

            orderbook.add_order(order).await?;
        }

        info!("Restored {} markets", orderbooks.len());
        Ok(())
    }

    async fn expire_orders(&self) -> Result<()> {
        let expired_orders = self.database.get_expired_orders().await?;
        let expired_count = expired_orders.len();
        let mut orderbooks = self.orderbooks.write().await;

        for mut order in expired_orders {
            // Remove from orderbook
            if let Some(market_orderbooks) = orderbooks.get_mut(&order.market_id) {
                if let Some(orderbook) = market_orderbooks.get_mut(&order.outcome) {
                    orderbook.remove_order(order.order_id).await?;
                }
            }

            // Update status in database
            order.status = OrderStatus::Expired;
            self.database.update_order(&order).await?;
        }

        if expired_count > 0 {
            info!("Expired {} orders", expired_count);
        }

        Ok(())
    }

    async fn cleanup_empty_price_levels(&self) -> Result<()> {
        let mut orderbooks = self.orderbooks.write().await;
        let mut cleaned_markets = 0;
        let mut cleaned_levels = 0;

        for (market_id, market_orderbooks) in orderbooks.iter_mut() {
            for (outcome, orderbook) in market_orderbooks.iter_mut() {
                let cleaned = orderbook.cleanup_empty_levels().await?;
                if cleaned > 0 {
                    cleaned_levels += cleaned;
                    debug!("Cleaned {} empty price levels in market {} outcome {}",
                          cleaned, market_id, outcome);
                }
            }
            cleaned_markets += 1;
        }

        if cleaned_levels > 0 {
            info!("Cleanup completed: {} empty price levels across {} markets",
                  cleaned_levels, cleaned_markets);
        }

        Ok(())
    }

    /// Execute regular orderbook matching (try existing liquidity first)
    async fn execute_regular_orderbook_matching(
        &self,
        working_order: &mut Order,
        market_orderbooks: &mut BTreeMap<u8, OrderBook>,
    ) -> Result<Vec<Trade>> {
        let orderbook = market_orderbooks
            .entry(working_order.outcome)
            .or_insert_with(OrderBook::new);

        // Attempt to match against existing orderbook liquidity
        let order_type = working_order.order_type.clone();
        let original_order = working_order.clone();

        let trades = match order_type {
            OrderType::Market => {
                orderbook.match_market_order(original_order).await?
            }
            OrderType::Limit | OrderType::GTC | OrderType::GTD => {
                // Standard limit order behavior
                orderbook.match_limit_order(original_order).await?
            }
            OrderType::FOK => {
                // Fill-or-Kill: must fill completely or not at all
                let potential_trades = orderbook.match_limit_order(original_order.clone()).await?;
                let total_filled: u128 = potential_trades.iter().map(|t| t.size).sum();
                if total_filled == original_order.remaining_size {
                    potential_trades
                } else {
                    // Cancel the order if it can't be filled completely
                    info!("FOK order {} cannot be filled completely, canceling", original_order.order_id);
                    Vec::new()
                }
            }
            OrderType::FAK => {
                // Fill-and-Kill: execute what's possible, cancel the rest
                let trades = orderbook.match_limit_order(original_order).await?;
                if !trades.is_empty() {
                    info!("FAK order {} partially filled with {} trades", working_order.order_id, trades.len());
                }
                trades
            }
        };

        // Update working order state based on trades
        for trade in &trades {
            working_order.remaining_size = working_order.remaining_size.saturating_sub(trade.size);
            working_order.filled_size = working_order.filled_size.saturating_add(trade.size);
            working_order.status = if working_order.remaining_size == 0 {
                OrderStatus::Filled
            } else {
                OrderStatus::PartiallyFilled
            };
        }

        if !trades.is_empty() {
            info!("✅ Regular orderbook matching: {} trades for order {}", trades.len(), working_order.order_id);
        }

        Ok(trades)
    }

    // Get collateral manager for external access
    pub fn get_collateral_manager(&self) -> &Arc<CollateralManager> {
        &self.collateral_manager
    }
}