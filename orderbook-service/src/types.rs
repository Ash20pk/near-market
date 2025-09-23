// Core types for the orderbook service

use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Order {
    pub order_id: Uuid,
    pub market_id: String,
    pub condition_id: String,
    pub user_account: String,      // NEAR account ID
    pub outcome: u8,               // 0=NO, 1=YES
    pub side: OrderSide,
    pub order_type: OrderType,
    pub price: u64,                // Price in 1/100000 of dollar (50000 = $0.50, 1000 = $0.01, 100 = $0.001)
    pub original_size: u128,       // Original order size
    pub remaining_size: u128,      // Unfilled amount
    pub filled_size: u128,         // Filled amount
    pub status: OrderStatus,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub solver_account: String,    // Which solver submitted this order
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum OrderType {
    Limit,    // Execute at specified price or better (same as GTC)
    Market,   // Execute immediately at best available price
    GTC,      // Good-Till-Canceled: stays active until manually canceled (same as Limit)
    FOK,      // Fill-or-Kill: execute completely immediately or cancel entirely
    GTD,      // Good-Till-Date: expires at specified date/time
    FAK,      // Fill-and-Kill: execute partial fills immediately, cancel remainder
}

/// Polymarket-style tick size configuration
pub struct TickSizeConfig {
    pub standard_tick: u64,    // 1000 = 0.01 (1 cent)
    pub fine_tick: u64,        // 100 = 0.001 (0.1 cent)
    pub fine_threshold_low: u64,  // 4000 = 0.04 (4 cents)
    pub fine_threshold_high: u64, // 96000 = 0.96 (96 cents)
}

impl Default for TickSizeConfig {
    fn default() -> Self {
        Self {
            standard_tick: 1000,     // 0.01 = 1 cent
            fine_tick: 100,          // 0.001 = 0.1 cent
            fine_threshold_low: 4000,  // 0.04 = 4 cents
            fine_threshold_high: 96000, // 0.96 = 96 cents
        }
    }
}

impl TickSizeConfig {
    /// Get appropriate tick size for a given price
    /// Prices are in basis points of cents (100000 = $1.00)
    pub fn get_tick_size(&self, price: u64) -> u64 {
        if price < self.fine_threshold_low || price > self.fine_threshold_high {
            self.fine_tick  // Use 0.1 cent precision at extremes
        } else {
            self.standard_tick  // Use 1 cent precision normally
        }
    }

    /// Validate and round price to appropriate tick size
    pub fn round_price(&self, price: u64) -> Result<u64, String> {
        if price == 0 {
            return Err("Price cannot be zero (use Market order instead)".to_string());
        }
        if price > 99999 {
            return Err("Price cannot exceed $0.99999".to_string());
        }

        let tick_size = self.get_tick_size(price);
        let rounded_price = (price / tick_size) * tick_size;

        // Ensure minimum price
        if rounded_price == 0 {
            Ok(tick_size) // Minimum is one tick
        } else {
            Ok(rounded_price)
        }
    }

    /// Check if price is valid (properly aligned to tick size)
    pub fn is_valid_price(&self, price: u64) -> bool {
        if price == 0 || price > 99999 {
            return false;
        }
        let tick_size = self.get_tick_size(price);
        price % tick_size == 0
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum OrderStatus {
    Pending,         // Waiting in orderbook
    PartiallyFilled, // Some fills executed
    Filled,          // Completely filled
    Cancelled,       // Cancelled by user/solver
    Expired,         // Expired due to time
    Failed,          // Settlement failed
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trade {
    pub trade_id: Uuid,
    pub market_id: String,
    pub condition_id: String,
    pub maker_order_id: Uuid,
    pub taker_order_id: Uuid,
    pub maker_account: String,
    pub taker_account: String,
    pub maker_side: OrderSide,
    pub taker_side: OrderSide,
    pub outcome: u8,
    pub price: u64,
    pub size: u128,
    pub trade_type: TradeType,
    pub executed_at: DateTime<Utc>,
    pub settlement_status: SettlementStatus,
    pub settlement_tx_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum TradeType {
    DirectMatch,    // Regular orderbook match
    Minting,        // Split USDC into YES+NO
    Burning,        // Merge YES+NO into USDC
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum SettlementStatus {
    Pending,        // Trade matched, awaiting settlement
    Settling,       // Settlement transaction submitted
    Settled,        // Successfully settled on-chain
    Failed,         // Settlement failed
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OrderbookSnapshot {
    pub market_id: String,
    pub outcome: u8,
    pub bids: Vec<PriceLevel>,  // Buy orders (highest price first)
    pub asks: Vec<PriceLevel>,  // Sell orders (lowest price first)
    pub last_trade_price: Option<u64>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PriceLevel {
    pub price: u64,
    pub size: u128,
    pub order_count: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MarketPrice {
    pub market_id: String,
    pub outcome: u8,
    pub bid: Option<u64>,       // Best buy price
    pub ask: Option<u64>,       // Best sell price
    pub mid: Option<u64>,       // Mid price (bid+ask)/2
    pub last: Option<u64>,      // Last trade price
    pub timestamp: DateTime<Utc>,
}

// API Request/Response types
#[derive(Debug, Deserialize)]
pub struct SubmitOrderRequest {
    pub market_id: String,
    pub user_account: String,
    pub solver_account: String,
    pub outcome: u8,
    pub side: OrderSide,
    pub order_type: OrderType,
    pub price: Option<u64>,     // None for market orders
    pub size: u128,
    pub expires_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct SubmitOrderResponse {
    pub order_id: Uuid,
    pub status: String,
    pub message: String,
    pub matches: Vec<TradeMatch>,
}

#[derive(Debug, Serialize)]
pub struct TradeMatch {
    pub trade_id: Uuid,
    pub counterparty: String,
    pub price: u64,
    pub size: u128,
    pub settlement_pending: bool,
}

#[derive(Debug, Deserialize)]
pub struct CancelOrderRequest {
    pub order_id: Uuid,
    pub user_account: String,
}

// WebSocket message types
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum WebSocketMessage {
    OrderbookUpdate {
        market_id: String,
        outcome: u8,
        snapshot: OrderbookSnapshot,
    },
    TradeExecuted {
        trade: Trade,
    },
    OrderUpdate {
        order_id: Uuid,
        status: OrderStatus,
        filled_size: u128,
    },
}

// Settlement batch for efficient on-chain execution
#[derive(Debug, Clone)]
pub struct SettlementBatch {
    pub batch_id: Uuid,
    pub trades: Vec<Trade>,
    pub total_gas_estimate: u64,
    pub created_at: DateTime<Utc>,
}

// ================================
// POLYMARKET-STYLE COLLATERAL SYSTEM
// ================================

/// User's collateral balance and reserved amounts per market
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollateralBalance {
    pub account_id: String,
    pub market_id: String,
    pub available_balance: u128,        // Free USDC available for new orders
    pub reserved_balance: u128,         // USDC reserved for open orders
    pub position_balance: u128,         // Value of outcome tokens held
    pub total_deposited: u128,          // Total USDC ever deposited
    pub total_withdrawn: u128,          // Total USDC ever withdrawn
    pub last_updated: DateTime<Utc>,
}

/// Collateral reservation for an order
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollateralReservation {
    pub reservation_id: Uuid,
    pub account_id: String,
    pub market_id: String,
    pub order_id: Uuid,
    pub reserved_amount: u128,          // USDC reserved for this order
    pub max_loss: u128,                 // Maximum possible loss
    pub side: OrderSide,
    pub price: u64,                     // Order price in cents
    pub size: u128,                     // Order size
    pub created_at: DateTime<Utc>,
}

/// Settlement instruction for collateral-based trades
#[derive(Debug, Clone)]
pub struct CollateralSettlement {
    pub settlement_id: Uuid,
    pub market_id: String,
    pub condition_id: String,
    pub trades: Vec<Trade>,
    pub total_collateral_required: u128,  // Total USDC needed to mint tokens
    pub net_transfers: Vec<CollateralTransfer>, // Net position changes
    pub tokens_to_mint: u128,             // Outcome token pairs to mint
    pub settlement_type: CollateralSettlementType,
}

#[derive(Debug, Clone)]
pub struct CollateralTransfer {
    pub from_account: String,
    pub to_account: String,
    pub outcome: u8,                    // 0=NO, 1=YES
    pub amount: u128,                   // Tokens to transfer
    pub net_usdc_flow: i128,           // Net USDC change (+ = receive, - = pay)
}

#[derive(Debug, Clone)]
pub enum CollateralSettlementType {
    PureMinting,     // Create new token pairs from USDC
    PureBurning,     // Burn token pairs back to USDC  
    TokenTransfer,   // Direct transfer of existing tokens
    MixedSettlement, // Combination of minting/burning/transfers
}

/// Market collateral requirements
#[derive(Debug, Clone)]
pub struct MarketCollateralConfig {
    pub market_id: String,
    pub min_collateral: u128,           // Minimum USDC to place orders
    pub margin_requirement: f64,        // Additional margin (e.g., 1.1 = 110% collateralization)
    pub max_leverage: f64,              // Maximum leverage allowed
}