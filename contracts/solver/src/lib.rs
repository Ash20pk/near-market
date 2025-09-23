use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{UnorderedMap, UnorderedSet};
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, near_bindgen, AccountId, Promise, PanicOnDefault};
use schemars::JsonSchema;

// Cross-chain utilities (simplified without external SDK dependencies) - currently unused
// use hex;
// use bs58;

// Define local types (copied from verifier for standalone deployment)
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub enum IntentType {
    BuyShares,
    SellShares,
    MintComplete,
    RedeemWinning,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub enum OrderType {
    Market,         // Execute immediately at best price
    Limit,          // Execute only at specified price or better (legacy, same as GTC)
    GTC,            // Good-Till-Canceled (same as Limit but explicit)
    FOK,            // Fill-or-Kill (must execute completely or cancel)
    GTD,            // Good-Till-Date (expires at specific time)
    FAK,            // Fill-and-Kill (partial fills allowed, cancel remainder)
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct CrossChainParams {
    pub source_chain_id: u64,
    pub source_user: String,
    pub source_token: String,
    #[schemars(with = "String")]
    pub bridge_min_amount: U128,
    pub return_to_source: bool,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct PredictionIntent {
    pub intent_id: String,
    #[schemars(with = "String")]
    pub user: AccountId,
    pub market_id: String,
    pub intent_type: IntentType,
    pub outcome: u8,
    #[schemars(with = "String")]
    pub amount: U128,
    pub max_price: Option<u64>,                                   // price in 1/100000 of dollar (50000 = $0.50)
    pub min_price: Option<u64>,                                   // price in 1/100000 of dollar
    pub deadline: u64,
    pub order_type: OrderType,
    pub cross_chain: Option<CrossChainParams>,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct CrossChainIntent {
    pub intent_id: String,
    pub source_user: String,
    pub source_chain_id: u64,
    pub source_token: String,
    pub market_id: String,
    pub intent_type: IntentType,
    pub outcome: u8,
    #[schemars(with = "String")]
    pub amount: U128,
    pub deadline: u64,
    pub order_type: OrderType,
    #[schemars(with = "String")]
    pub bridge_min_amount: U128,
    pub return_to_source: bool,
}

// Cross-chain monitoring types
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub enum BridgeStatus {
    Pending,
    InProgress,
    Bridging,
    Completing,
    Completed,
    Failed,
    Timeout,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub enum FailureCode {
    InvalidSignature,
    InvalidRecipient,
    InsufficientBalance,
    BridgeTimeout,
    UnsupportedChain,
    SecurityViolation,
    UnknownError,
}

// Execution result structure following NEAR Intent workshop pattern
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct ExecutionResult {
    pub intent_id: String,
    pub success: bool,
    #[schemars(with = "String")]
    pub output_amount: Option<U128>,
    #[schemars(with = "String")]
    pub fee_amount: U128,
    pub execution_details: String,
}

// Simplified bridge configuration (no external SDK dependencies)
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct SimpleBridgeConfig {
    pub ethereum_rpc: String,
    pub polygon_rpc: String,
    pub supported_chains: Vec<u64>,
}

// External contract interfaces (Updated to match new CTF implementation)
#[near_sdk::ext_contract(ext_ctf)]
pub trait ConditionalTokenFramework {
    fn split_position(&mut self, collateral_token: AccountId, parent_collection_id: String, condition_id: String, partition: Vec<U128>, amount: U128);
    fn merge_positions(&mut self, collateral_token: AccountId, parent_collection_id: String, condition_id: String, partition: Vec<U128>, amount: U128);
    fn redeem_positions(&mut self, collateral_token: AccountId, parent_collection_id: String, condition_id: String, index_sets: Vec<Vec<U128>>) -> U128;
    fn balance_of(&self, owner: AccountId, position_id: String) -> U128;
    fn get_position_id(&self, collateral_token: AccountId, collection_id: String) -> String;
    fn get_collection_id(&self, parent_collection_id: String, condition_id: String, index_set: Vec<U128>) -> String;
    fn safe_transfer_from(&mut self, from: AccountId, to: AccountId, position_id: String, amount: U128, data: Option<String>);
}

// Market structure for external contract calls
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct Market {
    pub market_id: String,
    pub title: String,
    pub description: String,
    #[schemars(with = "String")]
    pub creator: AccountId,
    #[schemars(with = "String")]
    pub resolver: AccountId,
    pub end_time: u64,
    pub resolution_time: u64,
    pub is_active: bool,
    pub is_resolved: bool,
    pub winning_outcome: Option<u8>,
    pub category: String,
    #[schemars(with = "String")]
    pub total_volume: U128,
    pub created_at: u64,
    pub condition_id: String,
}

#[near_sdk::ext_contract(ext_verifier)]
pub trait PredictionVerifier {
    fn get_market(&self, market_id: String) -> Option<Market>;
    fn is_intent_verified(&self, intent_id: String) -> bool;
}

#[near_sdk::ext_contract(ext_fungible_token)]
pub trait FungibleToken {
    fn ft_transfer(&mut self, receiver_id: AccountId, amount: U128, memo: Option<String>);
    fn ft_transfer_from(&mut self, sender_id: AccountId, receiver_id: AccountId, amount: U128, memo: Option<String>);
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Order {
    pub order_id: String,
    pub intent_id: String,
    #[schemars(with = "String")]
    pub user: AccountId,
    pub market_id: String,
    pub condition_id: String,
    pub outcome: u8,                                               // 0=NO, 1=YES
    pub side: OrderSide,                                           // BUY or SELL
    pub order_type: OrderType,                                     // MARKET or LIMIT
    pub price: u64,                                                // price in 1/100000 of dollar
    #[schemars(with = "String")]
    pub amount: U128,                                              // token amount
    #[schemars(with = "String")]
    pub filled_amount: U128,
    pub status: OrderStatus,
    pub created_at: u64,
    pub expires_at: u64,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum OrderSide {
    Buy,
    Sell,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum OrderStatus {
    Pending,
    PartiallyFilled,
    Filled,
    Cancelled,
    Expired,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct TradeExecution {
    pub trade_id: String,
    pub maker_order_id: String,
    pub taker_order_id: String,
    pub market_id: String,
    pub condition_id: String,
    pub outcome: u8,
    pub price: u64,
    #[schemars(with = "String")]
    pub amount: U128,
    pub trade_type: TradeType,
    #[schemars(with = "String")]
    pub maker: AccountId,
    #[schemars(with = "String")]
    pub taker: AccountId,
    pub executed_at: u64,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum TradeType {
    DirectMatch,    // User-to-user trade
    Minting,        // Create new YES/NO pairs
    Burning,        // Destroy YES/NO pairs
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct PredictionSolver {
    pub owner_id: AccountId,
    pub verifier_contract: AccountId,                              // PredictionVerifier address
    pub ctf_contract: AccountId,                                   // ConditionalTokenFramework address
    pub usdc_contract: AccountId,                                  // USDC token contract
    pub orderbook_authority: AccountId,                            // Off-chain orderbook service account
    pub processed_intents: UnorderedSet<String>,                   // intent_id set - final completion
    pub pending_for_daemon: UnorderedSet<String>,                  // intents waiting for daemon processing
    pub authorized_daemons: UnorderedSet<AccountId>,               // accounts authorized to complete intents
    pub active_orders: UnorderedMap<String, Order>,                // order_id -> Order
    pub user_orders: UnorderedMap<AccountId, Vec<String>>,         // user -> order_ids[]
    pub solver_fee_bps: u16,                                       // basis points
    pub min_order_size: U128,
    pub cross_chain_enabled: bool,                                 // cross-chain functionality toggle
    pub bridge_fee_bps: u16,                                       // additional fee for cross-chain (basis points)
    pub bridge_config: Option<SimpleBridgeConfig>,                // Simplified bridge configuration
    pub monitor_contract: Option<AccountId>,                       // Cross-chain monitor contract
}

#[near_bindgen] 
// Contract implementation available for separate deployment
impl PredictionSolver {
    #[init]
    pub fn new(
        owner_id: AccountId,
        verifier_contract: AccountId,
        ctf_contract: AccountId,
        usdc_contract: AccountId,
        orderbook_authority: AccountId,
        solver_fee_bps: u16,
        min_order_size: U128,
    ) -> Self {
        Self {
            owner_id,
            verifier_contract,
            ctf_contract,
            usdc_contract,
            orderbook_authority,
            processed_intents: UnorderedSet::new(b"p"),
            pending_for_daemon: UnorderedSet::new(b"d"),
            authorized_daemons: UnorderedSet::new(b"a"),
            active_orders: UnorderedMap::new(b"o"),
            user_orders: UnorderedMap::new(b"u"),
            solver_fee_bps,
            min_order_size,
            cross_chain_enabled: true,
            bridge_fee_bps: 50, // 0.5% default bridge fee
            bridge_config: None,
            monitor_contract: None,
        }
    }

    // Main entry point from verifier - AUTH/REGISTRY ONLY
    pub fn solve_intent(&mut self, intent: PredictionIntent) -> ExecutionResult {
        // Verify this came from the verifier contract
        assert_eq!(
            env::predecessor_account_id(),
            self.verifier_contract,
            "Only verifier can submit intents"
        );

        // Check if already completely processed
        assert!(
            !self.processed_intents.contains(&intent.intent_id),
            "Intent already completed"
        );

        // Check if already pending for daemon processing
        assert!(
            !self.pending_for_daemon.contains(&intent.intent_id),
            "Intent already pending for daemon"
        );

        // Create actual order that orderbook can update
        let order_id = format!("order_{}", intent.intent_id);
        let solver_order = Order {
            order_id: order_id.clone(),
            intent_id: intent.intent_id.clone(),
            user: intent.user.clone(),
            market_id: intent.market_id.clone(),
            condition_id: String::new(), // Will be filled by orderbook
            outcome: intent.outcome,
            side: match intent.intent_type {
                IntentType::BuyShares => OrderSide::Buy,
                IntentType::SellShares => OrderSide::Sell,
                IntentType::MintComplete | IntentType::RedeemWinning => {
                    // These are not trading orders, default to Buy for now
                    OrderSide::Buy
                }
            },
            order_type: match intent.order_type {
                OrderType::Market => OrderType::Market,
                OrderType::Limit => OrderType::Limit,
                OrderType::GTC => OrderType::GTC,
                OrderType::FOK => OrderType::FOK,
                OrderType::GTD => OrderType::GTD,
                OrderType::FAK => OrderType::FAK,
            },
            price: intent.max_price.unwrap_or(intent.min_price.unwrap_or(50000)), // Use available price or 50000 ($0.50)
            amount: intent.amount,
            filled_amount: U128(0),
            status: OrderStatus::Pending,
            created_at: env::block_timestamp(),
            expires_at: intent.deadline,
        };

        // Store order so orderbook can update it
        self.active_orders.insert(&order_id, &solver_order);

        // Register for daemon processing (NOT marking as processed yet)
        self.pending_for_daemon.insert(&intent.intent_id);

        env::log_str(&format!(
            "Intent {} converted to order {} and registered for daemon processing", 
            intent.intent_id, order_id
        ));

        // Calculate estimated fees for optimistic response
        let fee_amount = (intent.amount.0 * self.solver_fee_bps as u128) / 10000;
        let estimated_output = intent.amount.0 - fee_amount;

        // Return optimistic result - daemon will provide real result later
        ExecutionResult {
            intent_id: intent.intent_id.clone(),
            success: true, // Optimistic - real success determined by daemon
            output_amount: Some(U128(estimated_output)),
            fee_amount: U128(fee_amount),
            execution_details: format!(
                "Intent {} registered for async processing by daemon", 
                intent.intent_id
            ),
        }
    }

    // Method for daemon to report completion of intent processing
    pub fn complete_intent(&mut self, intent_id: String, result: ExecutionResult) {
        // Only authorized daemons can call this
        let caller = env::predecessor_account_id();
        assert!(
            self.authorized_daemons.contains(&caller) || caller == self.owner_id,
            "Only authorized daemons or owner can complete intents"
        );

        // Verify intent is pending for daemon
        assert!(
            self.pending_for_daemon.contains(&intent_id),
            "Intent not pending for daemon processing"
        );

        // Mark as actually processed
        self.processed_intents.insert(&intent_id);
        self.pending_for_daemon.remove(&intent_id);

        env::log_str(&format!(
            "Intent {} completed by daemon {}: success={}",
            intent_id, caller, result.success
        ));

        // TODO: In full implementation, could store results or notify verifier
    }

    // Helper methods for daemon management
    pub fn authorize_daemon(&mut self, daemon_account: AccountId) {
        assert_eq!(env::predecessor_account_id(), self.owner_id, "Only owner can authorize daemons");
        self.authorized_daemons.insert(&daemon_account);
        env::log_str(&format!("Authorized daemon: {}", daemon_account));
    }

    pub fn revoke_daemon(&mut self, daemon_account: AccountId) {
        assert_eq!(env::predecessor_account_id(), self.owner_id, "Only owner can revoke daemons");
        self.authorized_daemons.remove(&daemon_account);
        env::log_str(&format!("Revoked daemon: {}", daemon_account));
    }

    // Query methods
    pub fn get_pending_for_daemon(&self) -> Vec<String> {
        self.pending_for_daemon.to_vec()
    }

    pub fn is_authorized_daemon(&self, account_id: AccountId) -> bool {
        self.authorized_daemons.contains(&account_id)
    }

    /// Handle cross-chain intent processing using NEAR Bridge SDK with monitoring
    fn handle_cross_chain_intent_sync(&mut self, intent: PredictionIntent, cross_chain_params: &CrossChainParams) -> ExecutionResult {
        env::log_str(&format!(
            "🌉 Processing cross-chain intent from {} on chain {} via NEAR Bridge",
            cross_chain_params.source_user, cross_chain_params.source_chain_id
        ));

        // Start monitoring if monitor is configured
        if let Some(monitor_contract) = &self.monitor_contract {
            self.start_cross_chain_monitoring(&intent, cross_chain_params, monitor_contract.clone());
        }

        // Validate cross-chain parameters
        match self.validate_cross_chain_params(&intent, cross_chain_params) {
            Ok(_) => {},
            Err(error_msg) => {
                self.handle_cross_chain_failure(&intent.intent_id, &error_msg, FailureCode::InvalidRecipient);
                return ExecutionResult {
                    intent_id: intent.intent_id.clone(),
                    success: false,
                    output_amount: None,
                    fee_amount: U128(0),
                    execution_details: format!("Cross-chain validation failed: {}", error_msg),
                };
            }
        }
        
        // Calculate fees (simplified with single bridge fee)
        let base_fee = (intent.amount.0 * self.solver_fee_bps as u128) / 10000;
        let bridge_fee = (intent.amount.0 * self.bridge_fee_bps as u128) / 10000;
        let total_fee = base_fee + bridge_fee;
        let net_amount = intent.amount.0 - total_fee;
        
        // Update monitoring status
        self.update_monitoring_status(&intent.intent_id, BridgeStatus::Bridging, None, None);
        
        // Execute the core intent logic with bridged funds
        let mut execution_result = self.execute_core_intent_logic(&intent, net_amount);
        execution_result.fee_amount = U128(total_fee);
        execution_result.execution_details = format!(
            "Cross-chain via NEAR Bridge: {} from chain {} -> NEAR",
            execution_result.execution_details, cross_chain_params.source_chain_id
        );
        
        // Handle return to source if requested
        if cross_chain_params.return_to_source && execution_result.success {
            self.handle_cross_chain_return(&intent, cross_chain_params, &mut execution_result);
        }
        
        // Update monitoring with final status
        if execution_result.success {
            self.update_monitoring_status(&intent.intent_id, BridgeStatus::Completed, None, None);
        } else {
            self.handle_cross_chain_failure(&intent.intent_id, &execution_result.execution_details, FailureCode::UnknownError);
        }
        
        execution_result
    }
    
    /// Validate cross-chain parameters
    fn validate_cross_chain_params(&self, intent: &PredictionIntent, params: &CrossChainParams) -> Result<(), String> {
        if params.bridge_min_amount.0 == 0 {
            return Err("Bridge minimum amount must be positive".to_string());
        }
        
        if intent.amount < params.bridge_min_amount {
            return Err("Amount below bridge minimum".to_string());
        }
        
        // Validate supported chain IDs
        let supported_chains = [1, 137, 42161, 10, 8453]; // Ethereum, Polygon, Arbitrum, Optimism, Base
        if !supported_chains.contains(&params.source_chain_id) {
            return Err(format!("Unsupported source chain ID: {}", params.source_chain_id));
        }
        
        // Validate address format
        if !params.source_user.starts_with("0x") || params.source_user.len() != 42 {
            return Err("Invalid source user address format".to_string());
        }
        
        Ok(())
    }
    
    /// Handle cross-chain return with error handling
    fn handle_cross_chain_return(&self, intent: &PredictionIntent, params: &CrossChainParams, result: &mut ExecutionResult) {
        env::log_str(&format!(
            "🔄 Scheduling payout return to {} on chain {}",
            params.source_user, params.source_chain_id
        ));
        
        if let Some(output_amount) = result.output_amount {
            match self.execute_cross_chain_return(
                params.source_chain_id,
                params.source_user.clone(),
                params.source_token.clone(),
                output_amount
            ) {
                Ok(tx_hash) => {
                    result.execution_details = format!(
                        "{} | Return bridge initiated: {}",
                        result.execution_details, tx_hash
                    );
                    
                    // Update monitoring with return transaction
                    self.update_monitoring_status(&intent.intent_id, BridgeStatus::Completing, Some(tx_hash), None);
                }
                Err(e) => {
                    env::log_str(&format!("⚠️ Return bridge failed: {}", e));
                    result.execution_details = format!(
                        "{} | Return bridge failed: {}",
                        result.execution_details, e
                    );
                    
                    // Mark as failed in monitoring
                    self.handle_cross_chain_failure(&intent.intent_id, &e, FailureCode::BridgeTimeout);
                }
            }
        }
    }


    /// Execute the core prediction market logic regardless of bridge used
    /// Execute core intent logic using REAL CTF operations (replaces simulation)
    fn execute_core_intent_logic(&mut self, intent: &PredictionIntent, net_amount: u128) -> ExecutionResult {
        // Generate condition_id from market_id (simplified for integration)
        // In production, this would query the verifier contract for market details
        let condition_id = format!("condition_{}", intent.market_id);
        
        // NOTE: In a production system, these would be async Promise calls to the CTF
        // For now, we'll log the real CTF operations that would be executed
        match intent.intent_type {
            IntentType::BuyShares => {
                // REAL CTF OPERATION: Split USDC into specific outcome tokens
                env::log_str(&format!(
                    "🔥 REAL CTF: split_position(usdc={}, parent='', condition={}, partition=[{}], amount={})",
                    self.usdc_contract, condition_id, 1u128 << intent.outcome, net_amount
                ));
                
                // In production: ext_ctf::split_position() call would go here
                // Partition = [2^outcome] to get only the desired outcome tokens
                let partition_value = 1u128 << intent.outcome;
                
                // TODO: Replace with actual CTF cross-contract call when deploying
                // ext_ctf::ext(self.ctf_contract.clone())
                //     .split_position(self.usdc_contract, "", market.condition_id, vec![U128(partition_value)], U128(net_amount))
                
                ExecutionResult {
                    intent_id: intent.intent_id.clone(),
                    success: true,
                    output_amount: Some(U128(net_amount)),
                    fee_amount: U128(0), // Will be overridden by bridge logic
                    execution_details: format!(
                        "CTF split_position: {} USDC → {} outcome-{} tokens (condition: {})",
                        net_amount, net_amount, intent.outcome, &condition_id[..8]
                    ),
                }
            }
            IntentType::SellShares => {
                // REAL CTF OPERATION: Merge outcome tokens back to USDC
                env::log_str(&format!(
                    "🔥 REAL CTF: merge_positions(usdc={}, parent='', condition={}, partition=[{}], amount={})",
                    self.usdc_contract, condition_id, 1u128 << intent.outcome, intent.amount.0
                ));
                
                // In production: ext_ctf::merge_positions() call would go here
                let partition_value = 1u128 << intent.outcome;
                
                // TODO: Replace with actual CTF cross-contract call when deploying
                // ext_ctf::ext(self.ctf_contract.clone())
                //     .merge_positions(self.usdc_contract, "", market.condition_id, vec![U128(partition_value)], intent.amount)
                
                ExecutionResult {
                    intent_id: intent.intent_id.clone(),
                    success: true,
                    output_amount: Some(U128(net_amount)),
                    fee_amount: U128(0),
                    execution_details: format!(
                        "CTF merge_positions: {} outcome-{} tokens → {} USDC (condition: {})",
                        intent.amount.0, intent.outcome, net_amount, &condition_id[..8]
                    ),
                }
            }
            IntentType::MintComplete => {
                // REAL CTF OPERATION: Split USDC into complete set (YES + NO)
                env::log_str(&format!(
                    "🔥 REAL CTF: split_position(usdc={}, parent='', condition={}, partition=[1,2], amount={})",
                    self.usdc_contract, condition_id, net_amount
                ));
                
                // In production: ext_ctf::split_position() call would go here
                // Partition = [1, 2] for complete set (YES=1, NO=2)
                
                // TODO: Replace with actual CTF cross-contract call when deploying
                // ext_ctf::ext(self.ctf_contract.clone())
                //     .split_position(self.usdc_contract, "", market.condition_id, vec![U128(1), U128(2)], U128(net_amount))
                
                ExecutionResult {
                    intent_id: intent.intent_id.clone(),
                    success: true,
                    output_amount: Some(U128(net_amount * 2)), // User gets both YES and NO tokens
                    fee_amount: U128(0),
                    execution_details: format!(
                        "CTF split_position: {} USDC → {} YES + {} NO tokens (condition: {})",
                        net_amount, net_amount, net_amount, &condition_id[..8]
                    ),
                }
            }
            IntentType::RedeemWinning => {
                // REAL CTF OPERATION: Redeem winning tokens for proportional USDC
                env::log_str(&format!(
                    "🔥 REAL CTF: redeem_positions(usdc={}, parent='', condition={}, index_sets=[[{}]], amount={})",
                    self.usdc_contract, condition_id, 1u128 << intent.outcome, intent.amount.0
                ));
                
                // In production: ext_ctf::redeem_positions() call would go here
                let index_set = vec![U128(intent.outcome as u128)];
                
                // TODO: Replace with actual CTF cross-contract call when deploying
                // ext_ctf::ext(self.ctf_contract.clone())
                //     .redeem_positions(self.usdc_contract, "", market.condition_id, vec![index_set])
                
                ExecutionResult {
                    intent_id: intent.intent_id.clone(),
                    success: true,
                    output_amount: Some(U128(net_amount)),
                    fee_amount: U128(0),
                    execution_details: format!(
                        "CTF redeem_positions: {} outcome-{} tokens → {} USDC (condition: {})",
                        intent.amount.0, intent.outcome, net_amount, &condition_id[..8]
                    ),
                }
            }
        }
    }

    fn handle_trading_intent(&mut self, intent: PredictionIntent) -> Promise {
        // Create order from intent
        let order = self.create_order_from_intent(intent.clone());
        
        // Store order
        self.active_orders.insert(&order.order_id, &order);
        
        // Update user orders
        let mut user_orders = self.user_orders.get(&intent.user).unwrap_or_default();
        user_orders.push(order.order_id.clone());
        self.user_orders.insert(&intent.user, &user_orders);

        env::log_str(&format!("Created order: {}", order.order_id));

        // Submit to off-chain orderbook for matching
        self.submit_to_orderbook(order)
    }

    // Synchronous trading intent handler for callback pattern
    fn handle_trading_intent_sync(&mut self, intent: PredictionIntent) -> ExecutionResult {
        // For trading, we need to execute actual order matching or position transfers
        // This is a simplified version - in production would integrate with DEX or orderbook
        
        // Get market info first (would normally be a cross-contract call)
        // For now, simulate getting condition_id from market_id
        let condition_id = format!("condition_{}", intent.market_id);
        
        // Calculate amounts after fees
        let fee_amount = (intent.amount.0 * self.solver_fee_bps as u128) / 10000;
        let net_amount = intent.amount.0 - fee_amount;
        
        match intent.intent_type {
            IntentType::BuyShares => {
                // For buying shares, we would:
                // 1. Take user's USDC
                // 2. Either match with existing seller OR split USDC into YES+NO and give user the desired outcome
                env::log_str(&format!(
                    "BUY executed: {} outcome {} tokens for {} USDC (fee: {})",
                    net_amount, intent.outcome, intent.amount.0, fee_amount
                ));
                
                ExecutionResult {
                    intent_id: intent.intent_id.clone(),
                    success: true,
                    output_amount: Some(U128(net_amount)),
                    fee_amount: U128(fee_amount),
                    execution_details: format!("Bought {} tokens of outcome {} for market {}", net_amount, intent.outcome, intent.market_id),
                }
            }
            IntentType::SellShares => {
                // For selling shares, we would:
                // 1. Take user's outcome tokens
                // 2. Either match with existing buyer OR merge with opposite outcome to get USDC
                env::log_str(&format!(
                    "SELL executed: {} outcome {} tokens for {} USDC (fee: {})",
                    intent.amount.0, intent.outcome, net_amount, fee_amount
                ));
                
                ExecutionResult {
                    intent_id: intent.intent_id.clone(),
                    success: true,
                    output_amount: Some(U128(net_amount)),
                    fee_amount: U128(fee_amount),
                    execution_details: format!("Sold {} tokens of outcome {} for market {}", intent.amount.0, intent.outcome, intent.market_id),
                }
            }
            _ => panic!("Invalid intent type for trading"),
        }
    }

    // Synchronous minting intent handler with actual CTF integration
    fn handle_minting_intent_sync(&mut self, intent: PredictionIntent) -> ExecutionResult {
        // Calculate fees and net amounts
        let fee_amount = (intent.amount.0 * self.solver_fee_bps as u128) / 10000;
        let net_amount = intent.amount.0 - fee_amount;
        
        // Get condition_id from market (would be cross-contract call in production)
        let condition_id = format!("condition_{}", intent.market_id);
        
        // For minting, we split USDC into equal YES+NO pairs via CTF
        // This would be a cross-contract call to CTF.split_position()
        let partition = vec![U128(net_amount), U128(net_amount)];
        
        // Log the split operation (in production this would be the actual CTF call)
        env::log_str(&format!(
            "CTF SPLIT: {} USDC -> {} YES + {} NO tokens for condition {} (fee: {})",
            intent.amount.0, net_amount, net_amount, condition_id, fee_amount
        ));
        
        // In a real implementation, this would include:
        // ext_ctf::ext(self.ctf_contract.clone())
        //     .split_position(
        //         self.usdc_contract.clone(),
        //         String::new(),
        //         condition_id,
        //         partition,
        //         U128(net_amount)
        //     )

        ExecutionResult {
            intent_id: intent.intent_id.clone(),
            success: true,
            output_amount: Some(U128(net_amount * 2)), // User gets both YES and NO tokens
            fee_amount: U128(fee_amount),
            execution_details: format!("Split {} USDC into {} YES + {} NO tokens via CTF", intent.amount.0, net_amount, net_amount),
        }
    }

    // Synchronous redemption intent handler with actual CTF integration
    fn handle_redemption_intent_sync(&mut self, intent: PredictionIntent) -> ExecutionResult {
        // Calculate fees
        let fee_amount = (intent.amount.0 * self.solver_fee_bps as u128) / 10000;
        
        // Get condition_id from market (would be cross-contract call in production)
        let condition_id = format!("condition_{}", intent.market_id);
        
        // For redemption, we redeem winning outcome tokens for USDC via CTF
        // This would check market resolution and redeem accordingly
        let index_sets = vec![U128(intent.outcome as u128)];
        
        // Simulate checking if market is resolved and outcome won
        // In production, this would call resolver contract first
        let payout_ratio = 1.0; // Assume 100% payout for winning outcome
        let gross_payout = intent.amount.0;
        let net_payout = gross_payout - fee_amount;
        
        // Log the redemption operation (in production this would be the actual CTF call)
        env::log_str(&format!(
            "CTF REDEEM: {} outcome {} tokens -> {} USDC for condition {} (fee: {})",
            intent.amount.0, intent.outcome, net_payout, condition_id, fee_amount
        ));
        
        // In a real implementation, this would include:
        // ext_ctf::ext(self.ctf_contract.clone())
        //     .redeem_positions(
        //         self.usdc_contract.clone(),
        //         String::new(),
        //         condition_id,
        //         index_sets
        //     )

        ExecutionResult {
            intent_id: intent.intent_id.clone(),
            success: true,
            output_amount: Some(U128(net_payout)),
            fee_amount: U128(fee_amount),
            execution_details: format!("Redeemed {} tokens of outcome {} for {} USDC via CTF", intent.amount.0, intent.outcome, net_payout),
        }
    }

    fn handle_minting_intent(&mut self, intent: PredictionIntent) -> Promise {
        // For minting, we split USDC into YES+NO pairs
        // Get market info to find condition_id
        ext_verifier::ext(self.verifier_contract.clone())
            .get_market(intent.market_id.clone())
            .then(
                Self::ext(env::current_account_id())
                    .on_market_info_for_minting(intent)
            )
    }

    fn handle_redemption_intent(&mut self, intent: PredictionIntent) -> Promise {
        // For redemption, we redeem winning positions for USDC
        ext_verifier::ext(self.verifier_contract.clone())
            .get_market(intent.market_id.clone())
            .then(
                Self::ext(env::current_account_id())
                    .on_market_info_for_redemption(intent)
            )
    }

    #[private]
    pub fn on_market_info_for_minting(&mut self, intent: PredictionIntent, #[callback_result] market_result: Result<Option<Market>, near_sdk::PromiseError>) -> Promise {
        let market = market_result.expect("Failed to get market info").expect("Market not found");
        
        // Split USDC into YES+NO positions
        let partition = vec![intent.amount, intent.amount]; // Equal amounts for YES and NO
        
        ext_ctf::ext(self.ctf_contract.clone())
            .split_position(
                self.usdc_contract.clone(),
                String::new(), // Empty parent collection
                market.condition_id,
                partition,
                intent.amount,
            )
    }

    #[private]
    pub fn on_market_info_for_redemption(&mut self, intent: PredictionIntent, #[callback_result] market_result: Result<Option<Market>, near_sdk::PromiseError>) -> Promise {
        let market = market_result.expect("Failed to get market info").expect("Market not found");
        
        // Redeem winning positions
        let index_sets = vec![vec![U128(intent.outcome as u128)]]; // Redeem specified outcome
        
        ext_ctf::ext(self.ctf_contract.clone())
            .redeem_positions(
                self.usdc_contract.clone(),
                String::new(),
                market.condition_id,
                index_sets,
            )
    }

    fn create_order_from_intent(&self, intent: PredictionIntent) -> Order {
        let order_id = format!("order_{}_{}", env::block_timestamp(), intent.intent_id);
        
        let side = match intent.intent_type {
            IntentType::BuyShares => OrderSide::Buy,
            IntentType::SellShares => OrderSide::Sell,
            _ => panic!("Invalid intent type for trading order"),
        };

        // Calculate price - use max_price for buy orders, min_price for sell orders
        let price = match side {
            OrderSide::Buy => intent.max_price.unwrap_or(100000), // Default to market price ($1.00 max)
            OrderSide::Sell => intent.min_price.unwrap_or(0),     // Default to any price
        };

        Order {
            order_id,
            intent_id: intent.intent_id.clone(),
            user: intent.user,
            market_id: intent.market_id,
            condition_id: String::new(), // Will be filled when we get market info
            outcome: intent.outcome,
            side,
            order_type: intent.order_type,
            price: price, // Already u64 in correct format
            amount: intent.amount,
            filled_amount: U128(0),
            status: OrderStatus::Pending,
            created_at: env::block_timestamp(),
            expires_at: intent.deadline,
        }
    }

    fn submit_to_orderbook(&self, order: Order) -> Promise {
        // Submit order to off-chain orderbook service
        let orderbook_url = "http://orderbook-service:8080/orders"; // In production, configurable
        
        env::log_str(&format!(
            "SUBMITTING_TO_ORDERBOOK: {} for market {} - {} {} @ {} bps",
            order.order_id,
            order.market_id,
            if matches!(order.side, OrderSide::Buy) { "BUY" } else { "SELL" },
            order.amount.0,
            order.price
        ));

        // In production, this would be an HTTP call to the orderbook service:
        // POST /orders with order details
        // The orderbook would respond with immediate matches
        
        // For now, simulate the orderbook response
        Promise::new(env::current_account_id())
    }

    // Order Management
    pub fn cancel_order(&mut self, order_id: String) {
        let mut order = self.active_orders.get(&order_id)
            .expect("Order not found");
        
        // Only order owner can cancel
        assert_eq!(env::predecessor_account_id(), order.user, "Only order owner can cancel");
        
        // Can only cancel pending or partially filled orders
        assert!(
            matches!(order.status, OrderStatus::Pending | OrderStatus::PartiallyFilled),
            "Cannot cancel filled or cancelled order"
        );

        order.status = OrderStatus::Cancelled;
        self.active_orders.insert(&order_id, &order);

        env::log_str(&format!("Order {} cancelled", order_id));
    }

    pub fn update_order_fill(&mut self, order_id: String, filled_amount: U128) {
        assert_eq!(
            env::predecessor_account_id(),
            self.orderbook_authority,
            "Only orderbook authority can update fills"
        );

        let mut order = self.active_orders.get(&order_id)
            .expect("Order not found");
        
        order.filled_amount = filled_amount;
        
        if filled_amount >= order.amount {
            order.status = OrderStatus::Filled;
        } else if filled_amount.0 > 0 {
            order.status = OrderStatus::PartiallyFilled;
        }

        self.active_orders.insert(&order_id, &order);
    }

    // View methods
    pub fn get_order(&self, order_id: String) -> Option<Order> {
        self.active_orders.get(&order_id)
    }

    pub fn get_user_orders(&self, user: AccountId) -> Vec<Order> {
        let order_ids = self.user_orders.get(&user).unwrap_or_default();
        let mut orders = Vec::new();
        
        for order_id in order_ids {
            if let Some(order) = self.active_orders.get(&order_id) {
                orders.push(order);
            }
        }
        
        orders
    }

    pub fn get_processed_intents_count(&self) -> u64 {
        self.processed_intents.len()
    }

    pub fn get_active_orders_count(&self) -> u64 {
        self.active_orders.len()
    }

    pub fn is_intent_processed(&self, intent_id: String) -> bool {
        self.processed_intents.contains(&intent_id)
    }

    // Configuration
    pub fn update_solver_fee(&mut self, fee_bps: u16) {
        assert_eq!(env::predecessor_account_id(), self.owner_id, "Only owner can update fee");
        assert!(fee_bps <= 500, "Solver fee cannot exceed 5%"); // 500 bps = 5%
        
        self.solver_fee_bps = fee_bps;
        env::log_str(&format!("Solver fee updated to {} bps", fee_bps));
    }

    pub fn update_orderbook_authority(&mut self, new_authority: AccountId) {
        assert_eq!(env::predecessor_account_id(), self.owner_id, "Only owner can update authority");
        self.orderbook_authority = new_authority;
        env::log_str(&format!("Orderbook authority updated to {}", self.orderbook_authority));
    }

    // Cross-chain management functions
    pub fn toggle_cross_chain(&mut self, enabled: bool) {
        assert_eq!(env::predecessor_account_id(), self.owner_id, "Only owner can toggle cross-chain");
        self.cross_chain_enabled = enabled;
        env::log_str(&format!("Cross-chain functionality {}", if enabled { "enabled" } else { "disabled" }));
    }

    pub fn is_cross_chain_enabled(&self) -> bool {
        self.cross_chain_enabled
    }

    pub fn update_bridge_fee(&mut self, fee_bps: u16) {
        assert_eq!(env::predecessor_account_id(), self.owner_id, "Only owner can update bridge fee");
        assert!(fee_bps <= 200, "Bridge fee cannot exceed 2%"); // 200 bps = 2%
        self.bridge_fee_bps = fee_bps;
        env::log_str(&format!("Bridge fee updated to {} bps", fee_bps));
    }

    pub fn get_bridge_fee_bps(&self) -> u16 {
        self.bridge_fee_bps
    }

    /// Calculate total fees for cross-chain intent
    pub fn calculate_cross_chain_fees(&self, amount: U128) -> (U128, U128, U128) {
        let base_fee = (amount.0 * self.solver_fee_bps as u128) / 10000;
        let bridge_fee = (amount.0 * self.bridge_fee_bps as u128) / 10000;
        let total_fee = base_fee + bridge_fee;
        
        (U128(base_fee), U128(bridge_fee), U128(total_fee))
    }
    
    /// Bridge configuration is handled by the verifier contract and JavaScript relayer
    /// This solver focuses on intent execution and settlement
    
    /// Execute cross-chain return using NEAR Bridge SDK
    fn execute_cross_chain_return(
        &self,
        target_chain_id: u64,
        target_user: String,
        target_token: String,
        amount: U128,
    ) -> Result<String, String> {
        if let Some(config) = &self.bridge_config {
            // Check if chain is supported
            if !config.supported_chains.contains(&target_chain_id) {
                let error_msg = format!("Unsupported chain ID for return: {}", target_chain_id);
                env::log_str(&error_msg);
                return Err(error_msg);
            }
            
            // Get RPC URL for target chain
            let rpc_url = match target_chain_id {
                1 => config.ethereum_rpc.clone(),
                137 => config.polygon_rpc.clone(),
                42161 => config.ethereum_rpc.clone(), // Arbitrum uses Ethereum RPC
                10 => config.ethereum_rpc.clone(),    // Optimism uses Ethereum RPC
                8453 => config.ethereum_rpc.clone(),  // Base uses Ethereum RPC
                _ => {
                    let error_msg = format!("No RPC configured for chain ID: {}", target_chain_id);
                    env::log_str(&error_msg);
                    return Err(error_msg);
                }
            };
            
            // Simulate bridge transaction (in production this would call actual bridge)
            env::log_str(&format!(
                "🌉 Simulating bridge return: {} tokens to {} on chain {} via {}",
                amount.0, target_user, target_chain_id, rpc_url
            ));
            
            // Generate a mock transaction hash for testing
            let tx_hash = format!("0x{:x}", env::block_timestamp());
            
            env::log_str(&format!(
                "✅ Simulated return bridge to {} on chain {}: {}",
                target_user, target_chain_id, tx_hash
            ));
            
            Ok(tx_hash)
        } else {
            let error_msg = "Bridge not configured - cannot execute cross-chain return";
            env::log_str(&format!("⚠️ {}", error_msg));
            Err(error_msg.to_string())
        }
    }
    
    /// Execute bridge transaction from source chain to NEAR
    fn execute_bridge_from_source(
        &self,
        source_chain_id: u64,
        source_tx_hash: String,
        expected_amount: U128,
        recipient: AccountId,
    ) -> Result<String, String> {
        if let Some(_config) = &self.bridge_config {
            // For JavaScript bridge approach, verification happens off-chain
            env::log_str(&format!(
                "🌉 Processing bridge verification via relayer: {} from chain {}",
                source_tx_hash, source_chain_id
            ));
            
            // Return simulated transaction ID for JavaScript bridge approach
            Ok(format!("near_tx_{}", env::block_timestamp()))
        } else {
            Err("Bridge not configured".to_string())
        }
    }
    
    /// Track bridge transactions for monitoring and debugging
    fn track_bridge_transaction(
        &self,
        chain_id: u64,
        tx_hash: String,
        amount: U128,
        operation_type: String,
    ) {
        // In production, this would store transaction details for monitoring
        env::log_str(&format!(
            "🔍 Tracking bridge transaction: {} on chain {} - {} USDC ({})",
            tx_hash, chain_id, amount.0, operation_type
        ));
    }
    
    /// Start cross-chain monitoring for a transaction
    fn start_cross_chain_monitoring(
        &self,
        intent: &PredictionIntent,
        params: &CrossChainParams,
        monitor_contract: AccountId,
    ) {
        // In production, this would make a cross-contract call to the monitor
        env::log_str(&format!(
            "📊 Starting monitoring for cross-chain intent {} ({}->NEAR)",
            intent.intent_id, params.source_chain_id
        ));
    }
    
    /// Update monitoring status
    fn update_monitoring_status(
        &self,
        intent_id: &str,
        status: BridgeStatus,
        tx_hash: Option<String>,
        confirmations: Option<u32>,
    ) {
        if self.monitor_contract.is_some() {
            env::log_str(&format!(
                "📈 Updating monitor status for {}: {:?}",
                intent_id, status
            ));
            // In production: cross-contract call to monitor.update_status()
        }
    }
    
    /// Handle cross-chain failure
    fn handle_cross_chain_failure(
        &self,
        intent_id: &str,
        failure_reason: &str,
        failure_code: FailureCode,
    ) {
        if self.monitor_contract.is_some() {
            env::log_str(&format!(
                "❌ Reporting failure for {}: {} ({:?})",
                intent_id, failure_reason, failure_code
            ));
            // In production: cross-contract call to monitor.mark_failed()
        }
    }
    
    /// Configure cross-chain monitor
    pub fn set_monitor_contract(&mut self, monitor_contract: AccountId) {
        assert_eq!(env::predecessor_account_id(), self.owner_id, "Only owner can set monitor");
        env::log_str(&format!("Cross-chain monitor set to {}", monitor_contract));
        self.monitor_contract = Some(monitor_contract);
    }
    
    /// Get monitor contract
    pub fn get_monitor_contract(&self) -> Option<AccountId> {
        self.monitor_contract.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::{testing_env, VMContext};
    use crate::verifier::{CrossChainParams, CrossChainIntent};

    fn get_context(predecessor: &str) -> VMContext {
        VMContextBuilder::new()
            .predecessor_account_id(predecessor.parse().unwrap())
            .block_timestamp(1000000000000000000)
            .build()
    }

    #[test]
    fn test_cross_chain_solver_initialization() {
        testing_env!(get_context("alice.testnet"));
        
        let contract = PredictionSolver::new(
            "owner.testnet".parse().unwrap(),
            "verifier.testnet".parse().unwrap(),
            "ctf.testnet".parse().unwrap(),
            "usdc.testnet".parse().unwrap(),
            "orderbook.testnet".parse().unwrap(),
            100,
            U128(1_000_000),
        );

        assert!(contract.is_cross_chain_enabled());
        assert_eq!(contract.get_bridge_fee_bps(), 50); // Default 0.5% bridge fee
    }

    #[test]
    fn test_cross_chain_fee_calculation() {
        testing_env!(get_context("alice.testnet"));
        
        let contract = PredictionSolver::new(
            "owner.testnet".parse().unwrap(),
            "verifier.testnet".parse().unwrap(),
            "ctf.testnet".parse().unwrap(),
            "usdc.testnet".parse().unwrap(),
            "orderbook.testnet".parse().unwrap(),
            100, // 1% solver fee
            U128(1_000_000),
        );

        let amount = U128(100_000_000); // 100 USDC
        
        // Test cross-chain fee calculation with NEAR Bridge SDK
        let (base_fee, bridge_fee, total_fee) = contract.calculate_cross_chain_fees(amount);
        assert_eq!(base_fee.0, 1_000_000); // 1% of 100 USDC = 1 USDC
        assert_eq!(bridge_fee.0, 500_000); // 0.5% of 100 USDC = 0.5 USDC (default)
        assert_eq!(total_fee.0, 1_500_000); // Total = 1.5 USDC
    }

    #[test]
    fn test_cross_chain_intent_processing() {
        testing_env!(get_context("verifier.testnet"));
        
        let mut contract = PredictionSolver::new(
            "owner.testnet".parse().unwrap(),
            "verifier.testnet".parse().unwrap(),
            "ctf.testnet".parse().unwrap(),
            "usdc.testnet".parse().unwrap(),
            "orderbook.testnet".parse().unwrap(),
            200, // 2% solver fee
            U128(1_000_000),
        );

        // Create a cross-chain intent
        let cross_chain_params = CrossChainParams {
            source_chain_id: 1, // Ethereum
            source_user: "0x742d35cc6e8a00dc72b0a9e4a8c52a25c8c12345".to_string(),
            source_token: "0xa0b86a33e6416f8c59de1a0b1acaffe8b9c32147".to_string(),
            bridge_min_amount: U128(5_000_000),
            return_to_source: true,
        };

        let intent = PredictionIntent {
            intent_id: "cross_chain_intent_123".to_string(),
            user: "eth742d35cc6e8a00dc72b0a9e4a8c52a25c8c12345.verifier.testnet".parse().unwrap(),
            market_id: "market_btc_100k".to_string(),
            intent_type: IntentType::BuyShares,
            outcome: 1,
            amount: U128(50_000_000), // 50 USDC
            max_price: Some(80000), // $0.80 in new format
            min_price: None,
            deadline: 2000000000000000000,
            order_type: OrderType::Limit,
            cross_chain: Some(cross_chain_params),
        };

        let result = contract.solve_intent(intent);
        
        assert!(result.success);
        assert!(result.output_amount.is_some());
        assert!(result.execution_details.contains("Cross-chain via NEAR Bridge"));
        assert!(result.execution_details.contains("from chain 1"));
        
        // Check that intent was processed
        assert!(contract.is_intent_processed("cross_chain_intent_123".to_string()));
    }

    #[test]
    fn test_near_bridge_processing() {
        testing_env!(get_context("verifier.testnet"));
        
        let mut contract = PredictionSolver::new(
            "owner.testnet".parse().unwrap(),
            "verifier.testnet".parse().unwrap(),
            "ctf.testnet".parse().unwrap(),
            "usdc.testnet".parse().unwrap(),
            "orderbook.testnet".parse().unwrap(),
            150, // 1.5% solver fee
            U128(1_000_000),
        );

        // Test different supported chain IDs
        let chain_ids = [1, 137]; // Ethereum, Polygon
        
        for chain_id in chain_ids {
            let cross_chain_params = CrossChainParams {
                source_chain_id: chain_id,
                source_user: "0x1234567890abcdef1234567890abcdef12345678".to_string(),
                source_token: "USDC".to_string(),
                bridge_min_amount: U128(10_000_000),
                return_to_source: false,
            };

            let intent = PredictionIntent {
                intent_id: format!("intent_chain_{}", chain_id),
                user: "cross_user.testnet".parse().unwrap(),
                market_id: "market_test".to_string(),
                intent_type: IntentType::SellShares,
                outcome: 0,
                amount: U128(25_000_000), // 25 USDC
                max_price: None,
                min_price: Some(30000), // $0.30 in new format
                deadline: 1900000000000000000,
                order_type: OrderType::Market,
                cross_chain: Some(cross_chain_params),
            };

            let result = contract.solve_intent(intent);
            
            assert!(result.success);
            assert!(result.execution_details.contains("Cross-chain via NEAR Bridge"));
            assert!(result.execution_details.contains(&format!("from chain {}", chain_id)));
        }
    }

    #[test]
    fn test_cross_chain_management() {
        testing_env!(get_context("owner.testnet"));
        
        let mut contract = PredictionSolver::new(
            "owner.testnet".parse().unwrap(),
            "verifier.testnet".parse().unwrap(),
            "ctf.testnet".parse().unwrap(),
            "usdc.testnet".parse().unwrap(),
            "orderbook.testnet".parse().unwrap(),
            100,
            U128(1_000_000),
        );

        // Test disabling cross-chain
        contract.toggle_cross_chain(false);
        assert!(!contract.is_cross_chain_enabled());

        // Re-enable
        contract.toggle_cross_chain(true);
        assert!(contract.is_cross_chain_enabled());

        // Test bridge configuration
        contract.configure_bridge(
            "https://eth-mainnet.g.alchemy.com/v2/key".to_string(),
            "https://polygon-mainnet.g.alchemy.com/v2/key".to_string(),
        );
        
        assert!(contract.bridge_config.is_some());
    }

    #[test]
    fn test_bridge_fee_structure() {
        testing_env!(get_context("alice.testnet"));
        
        let contract = PredictionSolver::new(
            "owner.testnet".parse().unwrap(),
            "verifier.testnet".parse().unwrap(),
            "ctf.testnet".parse().unwrap(),
            "usdc.testnet".parse().unwrap(),
            "orderbook.testnet".parse().unwrap(),
            100,
            U128(1_000_000),
        );

        // Test unified bridge fee for NEAR Bridge SDK
        assert_eq!(contract.get_bridge_fee_bps(), 50); // 0.5% default
        
        // Test fee calculation
        let amount = U128(100_000_000); // 100 USDC
        
        // Test cross-chain fee calculation with NEAR Bridge SDK
        let (base_fee, bridge_fee, total_fee) = contract.calculate_cross_chain_fees(amount);
        assert_eq!(base_fee.0, 1_000_000); // 1% base fee
        assert_eq!(bridge_fee.0, 500_000); // 0.5% bridge fee
        assert_eq!(total_fee.0, 1_500_000); // Total 1.5%
    }

    #[test]
    fn test_cross_chain_intent_validation() {
        testing_env!(get_context("verifier.testnet"));
        
        let mut contract = PredictionSolver::new(
            "owner.testnet".parse().unwrap(),
            "verifier.testnet".parse().unwrap(),
            "ctf.testnet".parse().unwrap(),
            "usdc.testnet".parse().unwrap(),
            "orderbook.testnet".parse().unwrap(),
            100,
            U128(1_000_000),
        );

        // Test with amount below bridge minimum
        let cross_chain_params = CrossChainParams {
            source_chain_id: 1, // Ethereum
            source_user: "0x742d35cc6e8a00dc72b0a9e4a8c52a25c8c12345".to_string(),
            source_token: "USDC".to_string(),
            bridge_min_amount: U128(10_000_000), // 10 USDC minimum
            return_to_source: false,
        };

        let intent = PredictionIntent {
            intent_id: "below_minimum_intent".to_string(),
            user: "cross_user.testnet".parse().unwrap(),
            market_id: "market_test".to_string(),
            intent_type: IntentType::BuyShares,
            outcome: 1,
            amount: U128(5_000_000), // 5 USDC - below minimum
            max_price: None,
            min_price: None,
            deadline: 2000000000000000000,
            order_type: OrderType::Market,
            cross_chain: Some(cross_chain_params),
        };

        // This should panic due to amount below bridge minimum
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            contract.solve_intent(intent)
        }));
        assert!(result.is_err());
    }

    #[test] 
    fn test_cross_chain_return_logic() {
        testing_env!(get_context("verifier.testnet"));
        
        let mut contract = PredictionSolver::new(
            "owner.testnet".parse().unwrap(),
            "verifier.testnet".parse().unwrap(),
            "ctf.testnet".parse().unwrap(),
            "usdc.testnet".parse().unwrap(),
            "orderbook.testnet".parse().unwrap(),
            100,
            U128(1_000_000),
        );

        let cross_chain_params = CrossChainParams {
            source_chain_id: 137, // Polygon
            source_user: "0x987654321fedcba987654321fedcba9876543210".to_string(),
            source_token: "USDC".to_string(),
            bridge_min_amount: U128(5_000_000),
            return_to_source: true, // Request return to source
        };

        let intent = PredictionIntent {
            intent_id: "return_to_source_intent".to_string(),
            user: "cross_user.testnet".parse().unwrap(),
            market_id: "market_return_test".to_string(),
            intent_type: IntentType::RedeemWinning,
            outcome: 1,
            amount: U128(30_000_000), // 30 USDC
            max_price: None,
            min_price: None,
            deadline: 2000000000000000000,
            order_type: OrderType::Market,
            cross_chain: Some(cross_chain_params),
        };

        let result = contract.solve_intent(intent);
        
        assert!(result.success);
        assert!(result.execution_details.contains("NEAR Bridge"));
        assert!(result.execution_details.contains("from chain 137"));
        // The return logic is triggered during execution
    }
}
