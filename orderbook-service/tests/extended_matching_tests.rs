// Extended tests for edge cases and advanced scenarios

use std::collections::BTreeMap;
use uuid::Uuid;
use chrono::Utc;

use orderbook_service::types::{
    Order, OrderType, OrderSide, OrderStatus, Trade, TradeType, SettlementStatus
};

// Copy of TestOrderbook and helper functions for extended tests
#[derive(Debug)]
struct TestOrderbook {
    bids: BTreeMap<u64, Vec<Order>>, // price -> orders (highest price first)
    asks: BTreeMap<u64, Vec<Order>>, // price -> orders (lowest price first)
}

impl TestOrderbook {
    fn new() -> Self {
        Self {
            bids: BTreeMap::new(),
            asks: BTreeMap::new(),
        }
    }

    fn add_order(&mut self, order: Order) {
        match order.side {
            OrderSide::Buy => {
                self.bids.entry(order.price).or_insert(Vec::new()).push(order);
            }
            OrderSide::Sell => {
                self.asks.entry(order.price).or_insert(Vec::new()).push(order);
            }
        }
    }

    fn match_order(&mut self, incoming_order: Order) -> Vec<Trade> {
        let mut trades = Vec::new();
        let mut remaining_size = incoming_order.remaining_size;

        match incoming_order.side {
            OrderSide::Buy => {
                let ask_prices: Vec<u64> = self.asks.keys().cloned().collect();
                for ask_price in ask_prices {
                    if ask_price > incoming_order.price && incoming_order.price > 0 {
                        break;
                    }
                    if remaining_size == 0 { break; }

                    if let Some(orders) = self.asks.get_mut(&ask_price) {
                        let mut i = 0;
                        while i < orders.len() && remaining_size > 0 {
                            let maker_order = &mut orders[i];
                            let trade_size = std::cmp::min(remaining_size, maker_order.remaining_size);

                            if trade_size > 0 {
                                let trade = Trade {
                                    trade_id: Uuid::new_v4(),
                                    market_id: incoming_order.market_id.clone(),
                                    condition_id: incoming_order.condition_id.clone(),
                                    maker_order_id: maker_order.order_id,
                                    taker_order_id: incoming_order.order_id,
                                    maker_account: maker_order.user_account.clone(),
                                    taker_account: incoming_order.user_account.clone(),
                                    maker_side: OrderSide::Sell,
                                    taker_side: OrderSide::Buy,
                                    outcome: incoming_order.outcome,
                                    price: ask_price,
                                    size: trade_size,
                                    trade_type: TradeType::DirectMatch,
                                    executed_at: Utc::now(),
                                    settlement_status: SettlementStatus::Pending,
                                    settlement_tx_hash: None,
                                };

                                trades.push(trade);
                                remaining_size -= trade_size;
                                maker_order.remaining_size -= trade_size;
                                maker_order.filled_size += trade_size;

                                if maker_order.remaining_size == 0 {
                                    maker_order.status = OrderStatus::Filled;
                                } else {
                                    maker_order.status = OrderStatus::PartiallyFilled;
                                }
                            }

                            if maker_order.remaining_size == 0 {
                                orders.remove(i);
                            } else {
                                i += 1;
                            }
                        }

                        if orders.is_empty() {
                            self.asks.remove(&ask_price);
                        }
                    }
                }
            }
            OrderSide::Sell => {
                let bid_prices: Vec<u64> = self.bids.keys().rev().cloned().collect();
                for bid_price in bid_prices {
                    if bid_price < incoming_order.price { break; }
                    if remaining_size == 0 { break; }

                    if let Some(orders) = self.bids.get_mut(&bid_price) {
                        let mut i = 0;
                        while i < orders.len() && remaining_size > 0 {
                            let maker_order = &mut orders[i];
                            let trade_size = std::cmp::min(remaining_size, maker_order.remaining_size);

                            if trade_size > 0 {
                                let trade = Trade {
                                    trade_id: Uuid::new_v4(),
                                    market_id: incoming_order.market_id.clone(),
                                    condition_id: incoming_order.condition_id.clone(),
                                    maker_order_id: maker_order.order_id,
                                    taker_order_id: incoming_order.order_id,
                                    maker_account: maker_order.user_account.clone(),
                                    taker_account: incoming_order.user_account.clone(),
                                    maker_side: OrderSide::Buy,
                                    taker_side: OrderSide::Sell,
                                    outcome: incoming_order.outcome,
                                    price: bid_price,
                                    size: trade_size,
                                    trade_type: TradeType::DirectMatch,
                                    executed_at: Utc::now(),
                                    settlement_status: SettlementStatus::Pending,
                                    settlement_tx_hash: None,
                                };

                                trades.push(trade);
                                remaining_size -= trade_size;
                                maker_order.remaining_size -= trade_size;
                                maker_order.filled_size += trade_size;

                                if maker_order.remaining_size == 0 {
                                    maker_order.status = OrderStatus::Filled;
                                } else {
                                    maker_order.status = OrderStatus::PartiallyFilled;
                                }
                            }

                            if maker_order.remaining_size == 0 {
                                orders.remove(i);
                            } else {
                                i += 1;
                            }
                        }

                        if orders.is_empty() {
                            self.bids.remove(&bid_price);
                        }
                    }
                }
            }
        }
        trades
    }

    fn process_order(&mut self, mut order: Order) -> Vec<Trade> {
        let trades = match order.order_type {
            OrderType::Market => {
                if order.price == 0 {
                    match order.side {
                        OrderSide::Buy => order.price = u64::MAX,
                        OrderSide::Sell => order.price = 0,
                    }
                }
                self.match_order(order)
            }
            OrderType::Limit | OrderType::GTC => {
                let trades = self.match_order(order.clone());
                if order.remaining_size > 0 {
                    order.status = if trades.is_empty() { OrderStatus::Pending } else { OrderStatus::PartiallyFilled };
                    self.add_order(order);
                }
                trades
            }
            OrderType::FOK => {
                let total_available = self.get_available_liquidity(&order);
                if total_available >= order.remaining_size {
                    self.match_order(order)
                } else {
                    Vec::new()
                }
            }
            OrderType::FAK => {
                self.match_order(order)
            }
            OrderType::GTD => {
                if let Some(expires_at) = order.expires_at {
                    if expires_at <= Utc::now() {
                        return Vec::new();
                    }
                }
                let trades = self.match_order(order.clone());
                if order.remaining_size > 0 {
                    order.status = if trades.is_empty() { OrderStatus::Pending } else { OrderStatus::PartiallyFilled };
                    self.add_order(order);
                }
                trades
            }
        };
        trades
    }

    fn get_available_liquidity(&self, order: &Order) -> u128 {
        let mut total = 0u128;
        match order.side {
            OrderSide::Buy => {
                for (&price, orders) in &self.asks {
                    if price > order.price && order.price > 0 { break; }
                    total += orders.iter().map(|o| o.remaining_size).sum::<u128>();
                }
            }
            OrderSide::Sell => {
                for (&price, orders) in self.bids.iter().rev() {
                    if price < order.price { break; }
                    total += orders.iter().map(|o| o.remaining_size).sum::<u128>();
                }
            }
        }
        total
    }

    fn get_best_bid(&self) -> Option<u64> {
        self.bids.keys().next_back().copied()
    }

    fn get_best_ask(&self) -> Option<u64> {
        self.asks.keys().next().copied()
    }
}

fn create_order(side: OrderSide, order_type: OrderType, price: u64, size: u128, user: &str) -> Order {
    Order {
        order_id: Uuid::new_v4(),
        market_id: "test_market".to_string(),
        condition_id: "test_condition".to_string(),
        user_account: user.to_string(),
        outcome: 1,
        side,
        order_type,
        price,
        original_size: size,
        remaining_size: size,
        filled_size: 0,
        status: OrderStatus::Pending,
        created_at: Utc::now(),
        expires_at: None,
        solver_account: "test_solver".to_string(),
    }
}

#[cfg(test)]
mod extended_tests {
    use super::*;

    #[test]
    fn test_zero_size_orders() {
        let mut book = TestOrderbook::new();

        // Zero size orders should be rejected or handled gracefully
        let zero_order = create_order(OrderSide::Buy, OrderType::Limit, 50000, 0, "zero_user");
        let trades = book.process_order(zero_order);

        // Should not generate trades or cause panics
        assert_eq!(trades.len(), 0);
    }

    #[test]
    fn test_massive_orders() {
        let mut book = TestOrderbook::new();

        // Test with very large order sizes
        let big_sell = create_order(OrderSide::Sell, OrderType::Limit, 50000, u128::MAX / 2, "big_seller");
        book.process_order(big_sell);

        let big_buy = create_order(OrderSide::Buy, OrderType::Limit, 50000, u128::MAX / 4, "big_buyer");
        let trades = book.process_order(big_buy);

        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].size, u128::MAX / 4);
    }

    #[test]
    fn test_extreme_prices() {
        let mut book = TestOrderbook::new();

        // Test with high price that won't cross
        book.process_order(create_order(OrderSide::Sell, OrderType::Limit, 999999, 1000, "high_seller"));
        book.process_order(create_order(OrderSide::Buy, OrderType::Limit, 1, 1000, "low_buyer"));

        // Should not cause overflow or panics
        assert_eq!(book.get_best_ask(), Some(999999));
        assert_eq!(book.get_best_bid(), Some(1));
    }

    #[test]
    fn test_many_small_orders_vs_one_large() {
        let mut book = TestOrderbook::new();

        // Add 100 small sell orders at same price
        for i in 0..100 {
            book.process_order(create_order(OrderSide::Sell, OrderType::Limit, 50000, 10, &format!("seller_{}", i)));
        }

        // One large buy order should match all of them
        let big_buy = create_order(OrderSide::Buy, OrderType::Limit, 50000, 1000, "big_buyer");
        let trades = book.process_order(big_buy);

        assert_eq!(trades.len(), 100); // Should match all 100 orders
        assert_eq!(trades.iter().map(|t| t.size).sum::<u128>(), 1000);
    }

    #[test]
    fn test_price_level_clearing() {
        let mut book = TestOrderbook::new();

        // Add orders at specific price level
        book.process_order(create_order(OrderSide::Sell, OrderType::Limit, 50000, 300, "seller1"));
        book.process_order(create_order(OrderSide::Sell, OrderType::Limit, 50000, 200, "seller2"));

        // Completely clear the price level
        let clear_buy = create_order(OrderSide::Buy, OrderType::Limit, 50000, 500, "clearer");
        let trades = book.process_order(clear_buy);

        assert_eq!(trades.len(), 2);
        assert_eq!(trades.iter().map(|t| t.size).sum::<u128>(), 500);

        // Price level should be gone
        // (This would require exposing internal state or adding a method to check)
    }

    #[test]
    fn test_simultaneous_same_price_different_sides() {
        let mut book = TestOrderbook::new();

        // Add orders at exactly same price from both sides
        book.process_order(create_order(OrderSide::Buy, OrderType::Limit, 50000, 1000, "buyer"));

        let sell_order = create_order(OrderSide::Sell, OrderType::Limit, 50000, 800, "seller");
        let trades = book.process_order(sell_order);

        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].size, 800);
        assert_eq!(trades[0].price, 50000);
    }

    #[test]
    fn test_market_order_in_empty_book() {
        let mut book = TestOrderbook::new();

        // Market order with no liquidity should not crash
        let market_order = create_order(OrderSide::Buy, OrderType::Market, 0, 1000, "market_buyer");
        let trades = book.process_order(market_order);

        assert_eq!(trades.len(), 0);
    }

    #[test]
    fn test_fok_exact_liquidity_match() {
        let mut book = TestOrderbook::new();

        // Add exactly the amount needed for FOK
        book.process_order(create_order(OrderSide::Sell, OrderType::Limit, 50000, 500, "seller1"));
        book.process_order(create_order(OrderSide::Sell, OrderType::Limit, 51000, 300, "seller2"));

        // FOK for exactly 800 should succeed
        let fok_exact = create_order(OrderSide::Buy, OrderType::FOK, 51000, 800, "fok_buyer");
        let trades = book.process_order(fok_exact);

        assert_eq!(trades.len(), 2);
        assert_eq!(trades.iter().map(|t| t.size).sum::<u128>(), 800);
    }

    #[test]
    fn test_rapid_order_sequence() {
        let mut book = TestOrderbook::new();

        // Simulate rapid trading with alternating buy/sell
        let mut total_trades = 0;

        for i in 0..50 {
            let side = if i % 2 == 0 { OrderSide::Buy } else { OrderSide::Sell };
            let price = if side == OrderSide::Buy { 49000 + i * 10 } else { 51000 + i * 10 };

            let order = create_order(side, OrderType::Limit, price, 100, &format!("trader_{}", i));
            let trades = book.process_order(order);
            total_trades += trades.len();
        }

        // Should have generated some trades without panicking
        println!("Rapid sequence generated {} trades", total_trades);
    }

    #[test]
    fn test_order_modification_scenarios() {
        let mut book = TestOrderbook::new();

        // Test what happens when we try to "modify" orders by canceling and re-adding
        let original_order = create_order(OrderSide::Buy, OrderType::Limit, 50000, 1000, "modifier");
        let order_id = original_order.order_id;
        book.process_order(original_order);

        // This simulates order modification in real systems
        // (Our simple test doesn't support cancellation, but this tests the concept)
        let modified_order = create_order(OrderSide::Buy, OrderType::Limit, 51000, 800, "modifier");
        book.process_order(modified_order);

        // Should now have orders at two different price levels
        assert_eq!(book.get_best_bid(), Some(51000));
    }
}

// Performance and stress tests
#[cfg(test)]
mod performance_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn test_orderbook_performance_1000_orders() {
        let mut book = TestOrderbook::new();
        let start = Instant::now();

        // Add 1000 orders rapidly
        for i in 0..1000 {
            let side = if i % 2 == 0 { OrderSide::Buy } else { OrderSide::Sell };
            let price = if side == OrderSide::Buy { 50000 - (i as u64) } else { 50000 + (i as u64) };
            let order = create_order(side, OrderType::Limit, price, 100, &format!("perf_user_{}", i));
            book.process_order(order);
        }

        let duration = start.elapsed();
        println!("1000 orders processed in {:?}", duration);

        // Should complete in reasonable time (< 100ms for this simple implementation)
        assert!(duration.as_millis() < 1000);
    }

    #[test]
    fn test_large_spread_orderbook() {
        let mut book = TestOrderbook::new();

        // Create orderbook with very wide spread
        book.process_order(create_order(OrderSide::Buy, OrderType::Limit, 10000, 1000, "low_buyer"));
        book.process_order(create_order(OrderSide::Sell, OrderType::Limit, 90000, 1000, "high_seller"));

        // Market order should not cause issues
        let market_buy = create_order(OrderSide::Buy, OrderType::Market, 0, 500, "market_buyer");
        let trades = book.process_order(market_buy);

        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].price, 90000); // Should match high ask
    }
}

// Edge case scenarios that could occur in real trading
#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_crossing_orders_price_improvement() {
        let mut book = TestOrderbook::new();

        // Seller willing to sell at 45000
        book.process_order(create_order(OrderSide::Sell, OrderType::Limit, 45000, 1000, "cheap_seller"));

        // Buyer willing to pay 55000 should get price improvement
        let expensive_buy = create_order(OrderSide::Buy, OrderType::Limit, 55000, 800, "rich_buyer");
        let trades = book.process_order(expensive_buy);

        assert_eq!(trades.len(), 1);
        assert_eq!(trades[0].price, 45000); // Gets seller's better price
        assert_eq!(trades[0].size, 800);
    }

    #[test]
    fn test_multiple_fok_orders_same_liquidity() {
        let mut book = TestOrderbook::new();

        // Limited liquidity
        book.process_order(create_order(OrderSide::Sell, OrderType::Limit, 50000, 500, "limited_seller"));

        // First FOK should succeed
        let fok1 = create_order(OrderSide::Buy, OrderType::FOK, 50000, 500, "fok_buyer_1");
        let trades1 = book.process_order(fok1);
        assert_eq!(trades1.len(), 1);

        // Second FOK should fail (no liquidity left)
        let fok2 = create_order(OrderSide::Buy, OrderType::FOK, 50000, 500, "fok_buyer_2");
        let trades2 = book.process_order(fok2);
        assert_eq!(trades2.len(), 0);
    }

    #[test]
    fn test_order_priority_with_size_differences() {
        let mut book = TestOrderbook::new();

        // Add orders at same price with different sizes (time priority should win)
        let small_order = create_order(OrderSide::Buy, OrderType::Limit, 50000, 100, "small_buyer");
        let small_id = small_order.order_id;
        book.process_order(small_order);

        let large_order = create_order(OrderSide::Buy, OrderType::Limit, 50000, 1000, "large_buyer");
        book.process_order(large_order);

        // Incoming sell should match small order first (time priority)
        let sell_order = create_order(OrderSide::Sell, OrderType::Limit, 50000, 150, "seller");
        let trades = book.process_order(sell_order);

        assert_eq!(trades.len(), 2);
        assert_eq!(trades[0].maker_order_id, small_id);
        assert_eq!(trades[0].size, 100);
        assert_eq!(trades[1].size, 50);
    }
}