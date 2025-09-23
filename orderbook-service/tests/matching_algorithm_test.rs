// Pure matching algorithm tests - no blockchain, no client dependencies
// Tests core orderbook matching logic for all order types

use std::collections::BTreeMap;
use uuid::Uuid;
use chrono::Utc;

use orderbook_service::types::{
    Order, OrderType, OrderSide, OrderStatus, Trade, TradeType, SettlementStatus
};

// Simple in-memory orderbook for testing matching logic
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

    // Core matching algorithm - returns trades generated
    fn match_order(&mut self, incoming_order: Order) -> Vec<Trade> {
        let mut trades = Vec::new();
        let mut remaining_size = incoming_order.remaining_size;

        match incoming_order.side {
            OrderSide::Buy => {
                // Match against asks (sell orders) - lowest price first
                let ask_prices: Vec<u64> = self.asks.keys().cloned().collect();

                for ask_price in ask_prices {
                    if ask_price > incoming_order.price && incoming_order.price > 0 {
                        break; // Price too high for buy order
                    }
                    if remaining_size == 0 {
                        break;
                    }

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
                // Match against bids (buy orders) - highest price first
                let bid_prices: Vec<u64> = self.bids.keys().rev().cloned().collect();

                for bid_price in bid_prices {
                    if bid_price < incoming_order.price {
                        break; // Price too low for sell order
                    }
                    if remaining_size == 0 {
                        break;
                    }

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
                // Market orders match at any price
                if order.price == 0 {
                    // Set price to best available for matching
                    match order.side {
                        OrderSide::Buy => order.price = u64::MAX,
                        OrderSide::Sell => order.price = 0,
                    }
                }
                let trades = self.match_order(order.clone());
                trades
            }
            OrderType::Limit | OrderType::GTC => {
                // Limit orders try to match, then rest goes to book
                let trades = self.match_order(order.clone());
                if order.remaining_size > 0 {
                    order.status = if trades.is_empty() { OrderStatus::Pending } else { OrderStatus::PartiallyFilled };
                    self.add_order(order);
                }
                trades
            }
            OrderType::FOK => {
                // Fill-or-Kill: only execute if can fill completely
                let total_available = self.get_available_liquidity(&order);
                if total_available >= order.remaining_size {
                    self.match_order(order)
                } else {
                    Vec::new() // Rejected
                }
            }
            OrderType::FAK => {
                // Fill-and-Kill: fill what you can, cancel rest
                self.match_order(order)
            }
            OrderType::GTD => {
                // Good-Till-Date: check expiration first
                if let Some(expires_at) = order.expires_at {
                    if expires_at <= Utc::now() {
                        return Vec::new(); // Expired
                    }
                }
                // Otherwise behave like limit
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
                    if price > order.price && order.price > 0 {
                        break;
                    }
                    total += orders.iter().map(|o| o.remaining_size).sum::<u128>();
                }
            }
            OrderSide::Sell => {
                for (&price, orders) in self.bids.iter().rev() {
                    if price < order.price {
                        break;
                    }
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

// Test helper functions
fn create_order(
    side: OrderSide,
    order_type: OrderType,
    price: u64,
    size: u128,
    user: &str,
) -> Order {
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

// Tests
#[test]
fn test_limit_order_matching() {
    let mut book = TestOrderbook::new();

    // Place limit sell order
    let sell_order = create_order(OrderSide::Sell, OrderType::Limit, 50000, 1000, "seller");
    let trades = book.process_order(sell_order);
    assert_eq!(trades.len(), 0); // No match, goes to book

    // Place limit buy order that matches
    let buy_order = create_order(OrderSide::Buy, OrderType::Limit, 50000, 500, "buyer");
    let trades = book.process_order(buy_order);

    assert_eq!(trades.len(), 1);
    assert_eq!(trades[0].price, 50000);
    assert_eq!(trades[0].size, 500);
    assert_eq!(trades[0].maker_account, "seller");
    assert_eq!(trades[0].taker_account, "buyer");
}

#[test]
fn test_market_order_execution() {
    let mut book = TestOrderbook::new();

    // Place limit sell orders at different prices
    book.process_order(create_order(OrderSide::Sell, OrderType::Limit, 51000, 300, "seller1"));
    book.process_order(create_order(OrderSide::Sell, OrderType::Limit, 50000, 400, "seller2"));
    book.process_order(create_order(OrderSide::Sell, OrderType::Limit, 52000, 500, "seller3"));

    // Market buy order should match best prices first
    let market_buy = create_order(OrderSide::Buy, OrderType::Market, 0, 600, "buyer");
    let trades = book.process_order(market_buy);

    assert_eq!(trades.len(), 2); // Matches two best sellers
    assert_eq!(trades[0].price, 50000); // Best price first
    assert_eq!(trades[0].size, 400);
    assert_eq!(trades[1].price, 51000); // Second best price
    assert_eq!(trades[1].size, 200);
}

#[test]
fn test_fok_order_success() {
    let mut book = TestOrderbook::new();

    // Place enough liquidity
    book.process_order(create_order(OrderSide::Sell, OrderType::Limit, 50000, 400, "seller1"));
    book.process_order(create_order(OrderSide::Sell, OrderType::Limit, 51000, 300, "seller2"));

    // FOK order can be completely filled
    let fok_order = create_order(OrderSide::Buy, OrderType::FOK, 51000, 600, "buyer");
    let trades = book.process_order(fok_order);

    assert_eq!(trades.len(), 2);
    assert_eq!(trades.iter().map(|t| t.size).sum::<u128>(), 600);
}

#[test]
fn test_fok_order_rejection() {
    let mut book = TestOrderbook::new();

    // Place insufficient liquidity
    book.process_order(create_order(OrderSide::Sell, OrderType::Limit, 50000, 400, "seller1"));

    // FOK order cannot be completely filled
    let fok_order = create_order(OrderSide::Buy, OrderType::FOK, 50000, 600, "buyer");
    let trades = book.process_order(fok_order);

    assert_eq!(trades.len(), 0); // Order rejected
}

#[test]
fn test_fak_order_partial_fill() {
    let mut book = TestOrderbook::new();

    // Place partial liquidity
    book.process_order(create_order(OrderSide::Sell, OrderType::Limit, 50000, 400, "seller1"));

    // FAK order fills what it can
    let fak_order = create_order(OrderSide::Buy, OrderType::FAK, 50000, 600, "buyer");
    let trades = book.process_order(fak_order);

    assert_eq!(trades.len(), 1);
    assert_eq!(trades[0].size, 400); // Partial fill
}

#[test]
fn test_gtd_order_expiration() {
    let mut book = TestOrderbook::new();

    // Create expired GTD order
    let mut gtd_order = create_order(OrderSide::Buy, OrderType::GTD, 50000, 1000, "buyer");
    gtd_order.expires_at = Some(Utc::now() - chrono::Duration::seconds(10));

    let trades = book.process_order(gtd_order);
    assert_eq!(trades.len(), 0); // Should be rejected as expired
}

#[test]
fn test_price_time_priority() {
    let mut book = TestOrderbook::new();

    // Place two orders at same price (time priority matters)
    let order1 = create_order(OrderSide::Buy, OrderType::Limit, 50000, 300, "buyer1");
    let order1_id = order1.order_id;
    book.process_order(order1);

    let order2 = create_order(OrderSide::Buy, OrderType::Limit, 50000, 400, "buyer2");
    book.process_order(order2);

    // Incoming sell should match first order first (time priority)
    let sell_order = create_order(OrderSide::Sell, OrderType::Limit, 50000, 500, "seller");
    let trades = book.process_order(sell_order);

    assert_eq!(trades.len(), 2);
    assert_eq!(trades[0].maker_order_id, order1_id); // First order matched first
    assert_eq!(trades[0].size, 300);
    assert_eq!(trades[1].size, 200);
}

#[test]
fn test_price_improvement() {
    let mut book = TestOrderbook::new();

    // Place sell order at 52000
    book.process_order(create_order(OrderSide::Sell, OrderType::Limit, 52000, 1000, "seller"));

    // Buy order willing to pay 55000 should get price improvement
    let buy_order = create_order(OrderSide::Buy, OrderType::Limit, 55000, 500, "buyer");
    let trades = book.process_order(buy_order);

    assert_eq!(trades.len(), 1);
    assert_eq!(trades[0].price, 52000); // Gets better price than willing to pay
    assert_eq!(trades[0].size, 500);
}

#[test]
fn test_partial_fills() {
    let mut book = TestOrderbook::new();

    // Large buy order
    let big_buy = create_order(OrderSide::Buy, OrderType::Limit, 50000, 1000, "big_buyer");
    book.process_order(big_buy);

    // Multiple small sells
    let trades1 = book.process_order(create_order(OrderSide::Sell, OrderType::Limit, 50000, 200, "seller1"));
    let trades2 = book.process_order(create_order(OrderSide::Sell, OrderType::Limit, 50000, 300, "seller2"));
    let trades3 = book.process_order(create_order(OrderSide::Sell, OrderType::Limit, 50000, 600, "seller3"));

    assert_eq!(trades1.len(), 1);
    assert_eq!(trades1[0].size, 200);

    assert_eq!(trades2.len(), 1);
    assert_eq!(trades2[0].size, 300);

    assert_eq!(trades3.len(), 1);
    assert_eq!(trades3[0].size, 500); // Only 500 remaining from original 1000
}

#[test]
fn test_best_bid_ask() {
    let mut book = TestOrderbook::new();

    // Add various orders
    book.process_order(create_order(OrderSide::Buy, OrderType::Limit, 49000, 100, "buyer1"));
    book.process_order(create_order(OrderSide::Buy, OrderType::Limit, 48000, 200, "buyer2"));
    book.process_order(create_order(OrderSide::Sell, OrderType::Limit, 51000, 150, "seller1"));
    book.process_order(create_order(OrderSide::Sell, OrderType::Limit, 52000, 250, "seller2"));

    assert_eq!(book.get_best_bid(), Some(49000)); // Highest buy price
    assert_eq!(book.get_best_ask(), Some(51000)); // Lowest sell price
}

#[test]
fn test_all_order_types_compilation() {
    let mut book = TestOrderbook::new();

    // Test that all order types can be processed without errors
    let order_types = vec![
        OrderType::Limit,
        OrderType::Market,
        OrderType::GTC,
        OrderType::FOK,
        OrderType::GTD,
        OrderType::FAK,
    ];

    for order_type in order_types {
        let order = create_order(OrderSide::Buy, order_type, 50000, 100, "test_user");
        let trades = book.process_order(order);
        // Should not panic, trades may or may not be generated
        assert!(trades.len() <= 100); // Sanity check
    }
}

#[test]
fn test_crossing_orders() {
    let mut book = TestOrderbook::new();

    // Place sell at 50000
    book.process_order(create_order(OrderSide::Sell, OrderType::Limit, 50000, 500, "seller"));

    // Place buy at higher price (crosses)
    let buy_order = create_order(OrderSide::Buy, OrderType::Limit, 52000, 300, "buyer");
    let trades = book.process_order(buy_order);

    assert_eq!(trades.len(), 1);
    assert_eq!(trades[0].price, 50000); // Matches at seller's price
    assert_eq!(trades[0].size, 300);
}

#[test]
fn test_empty_orderbook() {
    let mut book = TestOrderbook::new();

    // Order in empty book should not match
    let order = create_order(OrderSide::Buy, OrderType::Limit, 50000, 1000, "buyer");
    let trades = book.process_order(order);

    assert_eq!(trades.len(), 0);
    assert_eq!(book.get_best_bid(), Some(50000)); // Order should be added to book
}

#[test]
fn test_order_type_behavior_differences() {
    let mut book1 = TestOrderbook::new();
    let mut book2 = TestOrderbook::new();

    // Test FOK behavior with insufficient liquidity
    book1.process_order(create_order(OrderSide::Sell, OrderType::Limit, 50000, 300, "seller"));
    let fok_order = create_order(OrderSide::Buy, OrderType::FOK, 50000, 500, "fok_buyer");
    let fok_trades = book1.process_order(fok_order);
    assert_eq!(fok_trades.len(), 0); // FOK rejected

    // Test FAK behavior with same liquidity (separate orderbook)
    book2.process_order(create_order(OrderSide::Sell, OrderType::Limit, 50000, 300, "seller"));
    let fak_order = create_order(OrderSide::Buy, OrderType::FAK, 50000, 500, "fak_buyer");
    let fak_trades = book2.process_order(fak_order);
    assert_eq!(fak_trades.len(), 1); // FAK partially filled
    assert_eq!(fak_trades[0].size, 300);
}

#[test]
fn test_comprehensive_orderbook_functionality() {
    let mut book = TestOrderbook::new();

    println!("=== Comprehensive Orderbook Test ===");

    // 1. Add initial liquidity with limit orders
    println!("1. Adding initial liquidity...");
    book.process_order(create_order(OrderSide::Sell, OrderType::Limit, 52000, 200, "seller_a"));
    book.process_order(create_order(OrderSide::Sell, OrderType::Limit, 51000, 300, "seller_b"));
    book.process_order(create_order(OrderSide::Buy, OrderType::Limit, 49000, 250, "buyer_a"));
    book.process_order(create_order(OrderSide::Buy, OrderType::Limit, 48000, 400, "buyer_b"));

    println!("   Best bid: {:?}, Best ask: {:?}", book.get_best_bid(), book.get_best_ask());

    // 2. Test market order execution
    println!("2. Testing market order...");
    let market_buy = create_order(OrderSide::Buy, OrderType::Market, 0, 250, "market_buyer");
    let trades = book.process_order(market_buy);
    println!("   Market buy generated {} trades, total size: {}",
             trades.len(), trades.iter().map(|t| t.size).sum::<u128>());
    assert_eq!(trades.len(), 1);
    assert_eq!(trades[0].price, 51000); // Should match best ask

    // 3. Test FOK success and failure
    println!("3. Testing FOK orders...");
    let fok_success = create_order(OrderSide::Buy, OrderType::FOK, 52000, 200, "fok_buyer_1");
    let trades = book.process_order(fok_success);
    println!("   FOK order (sufficient liquidity): {} trades", trades.len());
    assert!(trades.len() >= 1); // May span multiple price levels

    let fok_fail = create_order(OrderSide::Buy, OrderType::FOK, 52000, 500, "fok_buyer_2");
    let trades = book.process_order(fok_fail);
    println!("   FOK order (insufficient liquidity): {} trades", trades.len());
    assert_eq!(trades.len(), 0);

    // 4. Test FAK partial fill
    println!("4. Testing FAK partial fill...");
    book.process_order(create_order(OrderSide::Sell, OrderType::Limit, 53000, 150, "seller_c"));
    let fak_order = create_order(OrderSide::Buy, OrderType::FAK, 53000, 300, "fak_buyer");
    let trades = book.process_order(fak_order);
    let total_filled = trades.iter().map(|t| t.size).sum::<u128>();
    println!("   FAK order filled {} of 300 requested", total_filled);
    assert!(total_filled <= 300); // Should fill what's available

    // 5. Test price-time priority
    println!("5. Testing price-time priority...");
    let order1 = create_order(OrderSide::Buy, OrderType::Limit, 50000, 100, "priority_1");
    let order1_id = order1.order_id;
    book.process_order(order1);

    let order2 = create_order(OrderSide::Buy, OrderType::Limit, 50000, 200, "priority_2");
    book.process_order(order2);

    let sell_order = create_order(OrderSide::Sell, OrderType::Limit, 50000, 150, "seller_priority");
    let trades = book.process_order(sell_order);
    assert_eq!(trades.len(), 2);
    assert_eq!(trades[0].maker_order_id, order1_id); // First order gets priority
    assert_eq!(trades[0].size, 100);
    assert_eq!(trades[1].size, 50);

    // 6. Test GTD expiration
    println!("6. Testing GTD expiration...");
    let mut expired_order = create_order(OrderSide::Buy, OrderType::GTD, 55000, 1000, "expired_buyer");
    expired_order.expires_at = Some(Utc::now() - chrono::Duration::seconds(1));
    let trades = book.process_order(expired_order);
    println!("   Expired GTD order: {} trades", trades.len());
    assert_eq!(trades.len(), 0);

    println!("7. Final orderbook state:");
    println!("   Best bid: {:?}, Best ask: {:?}", book.get_best_bid(), book.get_best_ask());

    println!("=== All order types tested successfully! ===");
}