// Core orderbook matching logic - similar to Polymarket's approach

use std::collections::BTreeMap;
use uuid::Uuid;
use chrono::Utc;
use anyhow::Result;
use tracing::{debug, info};

use crate::types::{
    Order, Trade, OrderSide, OrderStatus, TradeType, SettlementStatus,
    OrderbookSnapshot, PriceLevel, MarketPrice
};

// Helper struct for atomic trade execution
#[derive(Clone)]
struct TradeParticipant {
    order_id: Uuid,
    user_account: String,
    side: OrderSide,
}

pub struct OrderBook {
    // Price -> Size aggregated levels for quick lookup
    bids: BTreeMap<u64, PriceLevel>,    // Buy orders (descending price)
    asks: BTreeMap<u64, PriceLevel>,    // Sell orders (ascending price)
    
    // Order ID -> Order for individual order management
    orders: BTreeMap<Uuid, Order>,
    
    // Price -> Vec<Order> for matching (orders at same price)
    bid_orders: BTreeMap<u64, Vec<Order>>,
    ask_orders: BTreeMap<u64, Vec<Order>>,
    
    // Market statistics
    last_trade_price: Option<u64>,
    total_volume: u128,
}

impl OrderBook {
    pub fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
            orders: BTreeMap::new(),
            bid_orders: BTreeMap::new(),
            ask_orders: BTreeMap::new(),
            last_trade_price: None,
            total_volume: 0,
        }
    }

    pub async fn add_order(&mut self, order: Order) -> Result<()> {
        let price = order.price;
        let size = order.remaining_size;
        
        // Add to orders map
        self.orders.insert(order.order_id, order.clone());

        // Add to appropriate price level
        match order.side {
            OrderSide::Buy => {
                // Add to bids
                let level = self.bids.entry(price).or_insert(PriceLevel {
                    price,
                    size: 0,
                    order_count: 0,
                });
                level.size += size;
                level.order_count += 1;

                // Add to bid orders for matching
                self.bid_orders.entry(price).or_default().push(order.clone());
            }
            OrderSide::Sell => {
                // Add to asks
                let level = self.asks.entry(price).or_insert(PriceLevel {
                    price,
                    size: 0,
                    order_count: 0,
                });
                level.size += size;
                level.order_count += 1;

                // Add to ask orders for matching
                self.ask_orders.entry(price).or_default().push(order.clone());
            }
        }

        debug!("Added order {} to orderbook at price {}", order.order_id, order.price);
        Ok(())
    }

    pub async fn remove_order(&mut self, order_id: Uuid) -> Result<()> {
        if let Some(order) = self.orders.remove(&order_id) {
            let price = order.price;
            let size = order.remaining_size;

            match order.side {
                OrderSide::Buy => {
                    // Update bids level
                    if let Some(level) = self.bids.get_mut(&price) {
                        level.size = level.size.saturating_sub(size);
                        level.order_count = level.order_count.saturating_sub(1);
                        
                        if level.order_count == 0 {
                            self.bids.remove(&price);
                        }
                    }

                    // Remove from bid orders
                    if let Some(orders) = self.bid_orders.get_mut(&price) {
                        orders.retain(|o| o.order_id != order_id);
                        if orders.is_empty() {
                            self.bid_orders.remove(&price);
                        }
                    }
                }
                OrderSide::Sell => {
                    // Update asks level
                    if let Some(level) = self.asks.get_mut(&price) {
                        level.size = level.size.saturating_sub(size);
                        level.order_count = level.order_count.saturating_sub(1);
                        
                        if level.order_count == 0 {
                            self.asks.remove(&price);
                        }
                    }

                    // Remove from ask orders
                    if let Some(orders) = self.ask_orders.get_mut(&price) {
                        orders.retain(|o| o.order_id != order_id);
                        if orders.is_empty() {
                            self.ask_orders.remove(&price);
                        }
                    }
                }
            }

            debug!("Removed order {} from orderbook", order_id);
        }

        Ok(())
    }

    pub async fn match_limit_order(&mut self, incoming_order: Order) -> Result<Vec<Trade>> {
        let mut trades = Vec::new();
        let mut remaining_order = incoming_order.clone();

        match incoming_order.side {
            OrderSide::Buy => {
                // Match against asks (sell orders), starting from lowest price
                while remaining_order.remaining_size > 0 {
                    let best_ask_info = self.asks.iter().next().map(|(&price, level)| (price, level.size));
                    
                    if let Some((best_ask_price, level_size)) = best_ask_info {
                        // Only match if our bid price >= ask price
                        if remaining_order.price >= best_ask_price {
                            if let Some(trade) = self.execute_match(
                                &mut remaining_order,
                                best_ask_price,
                                OrderSide::Sell
                            ).await? {
                                trades.push(trade);
                            } else {
                                // Check if the price level is empty (expired orders removed)
                                if level_size == 0 {
                                    continue; // Try again with the next price level
                                } else {
                                    break; // No more orders at this price level
                                }
                            }
                        } else {
                            break; // Price doesn't match
                        }
                    } else {
                        break; // No asks available
                    }
                }
            }
            OrderSide::Sell => {
                // Match against bids (buy orders), starting from highest price
                while remaining_order.remaining_size > 0 {
                    let best_bid_info = self.bids.iter().next_back().map(|(&price, level)| (price, level.size));
                    
                    if let Some((best_bid_price, level_size)) = best_bid_info {
                        // Only match if our ask price <= bid price
                        if remaining_order.price <= best_bid_price {
                            if let Some(trade) = self.execute_match(
                                &mut remaining_order,
                                best_bid_price,
                                OrderSide::Buy
                            ).await? {
                                trades.push(trade);
                            } else {
                                // Check if the price level is empty (expired orders removed)
                                if level_size == 0 {
                                    continue; // Try again with the next price level
                                } else {
                                    break; // No more orders at this price level
                                }
                            }
                        } else {
                            break; // Price doesn't match
                        }
                    } else {
                        break; // No bids available
                    }
                }
            }
        }

        // If there's remaining size, add to orderbook
        if remaining_order.remaining_size > 0 {
            self.add_order(remaining_order).await?;
        }

        info!("Limit order generated {} trades", trades.len());
        Ok(trades)
    }

    pub async fn match_market_order(&mut self, incoming_order: Order) -> Result<Vec<Trade>> {
        let mut trades = Vec::new();
        let mut remaining_order = incoming_order.clone();

        match incoming_order.side {
            OrderSide::Buy => {
                // Market buy: match against asks at any price
                while remaining_order.remaining_size > 0 {
                    let best_ask_info = self.asks.iter().next().map(|(&price, level)| (price, level.size));
                    
                    if let Some((best_ask_price, level_size)) = best_ask_info {
                        if let Some(trade) = self.execute_match(
                            &mut remaining_order,
                            best_ask_price,
                            OrderSide::Sell
                        ).await? {
                            trades.push(trade);
                        } else {
                            // Check if the price level is empty (expired orders removed)
                            if level_size == 0 {
                                continue; // Try again with the next price level
                            } else {
                                break; // No more liquidity
                            }
                        }
                    } else {
                        break; // No asks available
                    }
                }
            }
            OrderSide::Sell => {
                // Market sell: match against bids at any price
                while remaining_order.remaining_size > 0 {
                    let best_bid_info = self.bids.iter().next_back().map(|(&price, level)| (price, level.size));
                    
                    if let Some((best_bid_price, level_size)) = best_bid_info {
                        if let Some(trade) = self.execute_match(
                            &mut remaining_order,
                            best_bid_price,
                            OrderSide::Buy
                        ).await? {
                            trades.push(trade);
                        } else {
                            // Check if the price level is empty (expired orders removed)
                            if level_size == 0 {
                                continue; // Try again with the next price level
                            } else {
                                break; // No more liquidity
                            }
                        }
                    } else {
                        break; // No bids available
                    }
                }
            }
        }

        // Market orders don't go into the book - they either fill or fail
        if remaining_order.remaining_size > 0 {
            info!("Market order partially filled: {} remaining", remaining_order.remaining_size);
        }

        info!("Market order generated {} trades", trades.len());
        Ok(trades)
    }

    async fn execute_match(
        &mut self,
        taker_order: &mut Order,
        maker_price: u64,
        maker_side: OrderSide,
    ) -> Result<Option<Trade>> {
        // Check for expired orders first and clean them up
        self.remove_expired_orders_at_price(maker_price, &maker_side).await?;

        // Get maker orders at this price level
        let maker_orders = match maker_side {
            OrderSide::Buy => self.bid_orders.get_mut(&maker_price),
            OrderSide::Sell => self.ask_orders.get_mut(&maker_price),
        };

        if let Some(orders) = maker_orders {
            if let Some(maker_order) = orders.first_mut() {

                // Calculate trade size (minimum of both orders)
                let trade_size = std::cmp::min(
                    taker_order.remaining_size,
                    maker_order.remaining_size
                );

                if trade_size == 0 {
                    return Ok(None);
                }

                // Create immutable snapshot for trade creation to avoid borrow conflicts
                let maker_snapshot = TradeParticipant {
                    order_id: maker_order.order_id,
                    user_account: maker_order.user_account.clone(),
                    side: maker_order.side.clone(),
                };

                // Create trade - for Polymarket-style CLOB, always mint from collateral
                let trade = Trade {
                    trade_id: Uuid::new_v4(),
                    market_id: taker_order.market_id.clone(),
                    condition_id: taker_order.condition_id.clone(),
                    maker_order_id: maker_snapshot.order_id,
                    taker_order_id: taker_order.order_id,
                    maker_account: maker_snapshot.user_account,
                    taker_account: taker_order.user_account.clone(),
                    maker_side: maker_snapshot.side,
                    taker_side: taker_order.side.clone(),
                    outcome: taker_order.outcome,
                    price: maker_price, // Trade executes at maker's price
                    size: trade_size,
                    trade_type: TradeType::Minting, // Polymarket style: mint from collateral
                    executed_at: Utc::now(),
                    settlement_status: SettlementStatus::Pending,
                    settlement_tx_hash: None,
                };

                // Atomically update both orders
                let (taker_new_status, maker_new_status, should_remove_maker) = {
                    // Update order sizes
                    taker_order.remaining_size -= trade_size;
                    taker_order.filled_size += trade_size;
                    maker_order.remaining_size -= trade_size;
                    maker_order.filled_size += trade_size;

                    // Calculate new statuses
                    let taker_status = if taker_order.remaining_size == 0 {
                        OrderStatus::Filled
                    } else {
                        OrderStatus::PartiallyFilled
                    };

                    let maker_status = if maker_order.remaining_size == 0 {
                        OrderStatus::Filled
                    } else {
                        OrderStatus::PartiallyFilled
                    };

                    let should_remove = maker_order.remaining_size == 0;
                    (taker_status, maker_status, should_remove)
                };

                // Apply status updates
                taker_order.status = taker_new_status;
                maker_order.status = maker_new_status;

                let maker_order_id = maker_order.order_id;

                // Remove filled order from the book atomically
                if should_remove_maker {
                    orders.remove(0);
                    self.orders.remove(&maker_order_id);
                }

                // Update price levels atomically
                self.update_price_level_after_trade(maker_price, trade_size, &maker_side).await?;

                // Update market statistics
                self.last_trade_price = Some(maker_price);
                self.total_volume = self.total_volume.saturating_add(trade_size);

                debug!("Executed trade: {} @ {} between {} and {}",
                    trade_size, maker_price, trade.maker_account, trade.taker_account);

                return Ok(Some(trade));
            }
        }

        Ok(None)
    }

    async fn update_price_level_after_trade(
        &mut self,
        price: u64,
        trade_size: u128,
        side: &OrderSide,
    ) -> Result<()> {
        match side {
            OrderSide::Buy => {
                if let Some(level) = self.bids.get_mut(&price) {
                    level.size = level.size.saturating_sub(trade_size);
                    level.order_count = level.order_count.saturating_sub(1);

                    // Clean up empty price levels to prevent memory leaks
                    if level.size == 0 || level.order_count == 0 {
                        self.bids.remove(&price);
                        self.bid_orders.remove(&price);
                        debug!("Cleaned up empty bid level at price {}", price);
                    }
                }
            }
            OrderSide::Sell => {
                if let Some(level) = self.asks.get_mut(&price) {
                    level.size = level.size.saturating_sub(trade_size);
                    level.order_count = level.order_count.saturating_sub(1);

                    // Clean up empty price levels to prevent memory leaks
                    if level.size == 0 || level.order_count == 0 {
                        self.asks.remove(&price);
                        self.ask_orders.remove(&price);
                        debug!("Cleaned up empty ask level at price {}", price);
                    }
                }
            }
        }
        Ok(())
    }

    async fn remove_expired_orders_at_price(&mut self, price: u64, side: &OrderSide) -> Result<()> {
        let now = Utc::now();
        let orders_to_remove: Vec<Uuid> = {
            let orders = match side {
                OrderSide::Buy => self.bid_orders.get(&price),
                OrderSide::Sell => self.ask_orders.get(&price),
            };
            
            if let Some(orders) = orders {
                orders.iter()
                    .filter(|order| {
                        if let Some(expires_at) = order.expires_at {
                            now > expires_at
                        } else {
                            false
                        }
                    })
                    .map(|order| order.order_id)
                    .collect()
            } else {
                Vec::new()
            }
        };
        
        // Remove expired orders
        for order_id in orders_to_remove {
            self.remove_order(order_id).await?;
        }
        
        Ok(())
    }

    /// Get orders at specific price and side for complementary matching
    pub async fn get_orders_by_price_and_side(&mut self, price: u64, side: OrderSide) -> Result<Option<Order>> {
        debug!("🔍 Searching for orders: price={}, side={:?}", price, side);

        let orders = match side {
            OrderSide::Buy => {
                debug!("📊 Available bid prices: {:?}", self.bid_orders.keys().collect::<Vec<_>>());
                self.bid_orders.get_mut(&price)
            },
            OrderSide::Sell => {
                debug!("📊 Available ask prices: {:?}", self.ask_orders.keys().collect::<Vec<_>>());
                self.ask_orders.get_mut(&price)
            },
        };

        if let Some(order_list) = orders {
            debug!("✅ Found {} orders at price {}", order_list.len(), price);
            if let Some(order) = order_list.first() {
                debug!("🎯 Returning first order: {} by {}", order.order_id, order.user_account);
                return Ok(Some(order.clone()));
            }
        } else {
            debug!("❌ No orders found at price {} for side {:?}", price, side);
        }

        Ok(None)
    }

    /// Update order size for partially filled orders in complementary matching
    pub async fn update_order_size(&mut self, order_id: Uuid, new_remaining_size: u128) -> Result<()> {
        if let Some(order) = self.orders.get_mut(&order_id) {
            let size_diff = order.remaining_size.saturating_sub(new_remaining_size);
            order.remaining_size = new_remaining_size;

            // Update price level accordingly
            let price = order.price;
            let side = &order.side;

            match side {
                OrderSide::Buy => {
                    if let Some(level) = self.bids.get_mut(&price) {
                        level.size = level.size.saturating_sub(size_diff);
                    }
                }
                OrderSide::Sell => {
                    if let Some(level) = self.asks.get_mut(&price) {
                        level.size = level.size.saturating_sub(size_diff);
                    }
                }
            }

            debug!("Updated order {} size to {}", order_id, new_remaining_size);
        }
        Ok(())
    }

    /// Remove a specific order from the orderbook (used in complementary matching)
    pub async fn remove_specific_order(&mut self, order_id: Uuid, price: u64, side: OrderSide) -> Result<()> {
        let orders = match side {
            OrderSide::Buy => self.bid_orders.get_mut(&price),
            OrderSide::Sell => self.ask_orders.get_mut(&price),
        };

        if let Some(order_list) = orders {
            order_list.retain(|o| o.order_id != order_id);
            if order_list.is_empty() {
                match side {
                    OrderSide::Buy => {
                        self.bid_orders.remove(&price);
                    }
                    OrderSide::Sell => {
                        self.ask_orders.remove(&price);
                    }
                }
            }
        }

        // Also remove from the main orders map and price levels
        self.orders.remove(&order_id);

        // Update price levels
        match side {
            OrderSide::Buy => {
                if let Some(level) = self.bids.get_mut(&price) {
                    level.order_count = level.order_count.saturating_sub(1);
                    if level.order_count == 0 {
                        self.bids.remove(&price);
                    }
                }
            }
            OrderSide::Sell => {
                if let Some(level) = self.asks.get_mut(&price) {
                    level.order_count = level.order_count.saturating_sub(1);
                    if level.order_count == 0 {
                        self.asks.remove(&price);
                    }
                }
            }
        }

        Ok(())
    }

    pub async fn get_snapshot(&self, market_id: &str, outcome: u8) -> Result<OrderbookSnapshot> {
        // Convert bids to descending order (highest price first)
        let bids: Vec<PriceLevel> = self.bids
            .iter()
            .rev()
            .map(|(_, level)| (*level).clone())
            .collect();

        // Convert asks to ascending order (lowest price first)
        let asks: Vec<PriceLevel> = self.asks
            .iter()
            .map(|(_, level)| (*level).clone())
            .collect();

        Ok(OrderbookSnapshot {
            market_id: market_id.to_string(),
            outcome,
            bids,
            asks,
            last_trade_price: self.last_trade_price,
            timestamp: Utc::now(),
        })
    }

    pub async fn get_market_price(&self, market_id: &str, outcome: u8) -> Result<MarketPrice> {
        let bid = self.bids.keys().next_back().copied();
        let ask = self.asks.keys().next().copied();
        
        let mid = match (bid, ask) {
            (Some(b), Some(a)) => Some((b + a) / 2),
            _ => None,
        };

        Ok(MarketPrice {
            market_id: market_id.to_string(),
            outcome,
            bid,
            ask,
            mid,
            last: self.last_trade_price,
            timestamp: Utc::now(),
        })
    }

    /// Cleanup empty price levels to prevent memory leaks
    pub async fn cleanup_empty_levels(&mut self) -> Result<usize> {
        let mut cleaned_count = 0;

        // Clean empty bid levels
        let empty_bid_prices: Vec<u64> = self.bids
            .iter()
            .filter(|(_, level)| level.size == 0 || level.order_count == 0)
            .map(|(&price, _)| price)
            .collect();

        for price in empty_bid_prices {
            self.bids.remove(&price);
            self.bid_orders.remove(&price);
            cleaned_count += 1;
        }

        // Clean empty ask levels
        let empty_ask_prices: Vec<u64> = self.asks
            .iter()
            .filter(|(_, level)| level.size == 0 || level.order_count == 0)
            .map(|(&price, _)| price)
            .collect();

        for price in empty_ask_prices {
            self.asks.remove(&price);
            self.ask_orders.remove(&price);
            cleaned_count += 1;
        }

        // Clean orphaned order vectors
        let orphaned_bid_prices: Vec<u64> = self.bid_orders
            .iter()
            .filter(|(_, orders)| orders.is_empty())
            .map(|(&price, _)| price)
            .collect();

        for price in orphaned_bid_prices {
            self.bid_orders.remove(&price);
            cleaned_count += 1;
        }

        let orphaned_ask_prices: Vec<u64> = self.ask_orders
            .iter()
            .filter(|(_, orders)| orders.is_empty())
            .map(|(&price, _)| price)
            .collect();

        for price in orphaned_ask_prices {
            self.ask_orders.remove(&price);
            cleaned_count += 1;
        }

        Ok(cleaned_count)
    }
}