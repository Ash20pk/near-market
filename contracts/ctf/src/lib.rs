use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{UnorderedMap, UnorderedSet};
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault};
use near_sdk::env::sha256;
use schemars::JsonSchema;

// Core CTF data structures following Polymarket/Gnosis CTF architecture

/// Represents a condition in the CTF system
/// Conditions are prepared by oracles and define the possible outcomes
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct Condition {
    #[schemars(with = "String")]
    pub oracle: AccountId,
    pub question_id: String,
    pub outcome_slot_count: u8,
    #[schemars(with = "Option<Vec<String>>")]
    pub payout_numerators: Option<Vec<U128>>,  // Set when resolved
    #[schemars(with = "Option<String>")]
    pub payout_denominator: Option<U128>,      // Set when resolved
}

/// Position represents a conditional token position
/// Each position corresponds to a specific outcome of a condition
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct Position {
    pub position_id: String,
    #[schemars(with = "String")]
    pub collateral_token: AccountId,
    pub collection_id: String,
    pub condition_id: String,
    #[schemars(with = "Vec<String>")]
    pub index_set: Vec<U128>,
}

/// Collection represents a set of positions for a given condition
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema, Clone, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct Collection {
    pub collection_id: String,
    pub parent_collection_id: String,
    pub condition_id: String,
    #[schemars(with = "Vec<String>")]
    pub index_set: Vec<U128>,
}

/// External contract interface for fungible tokens (USDC, etc.)
#[near_sdk::ext_contract(ext_fungible_token)]
pub trait FungibleToken {
    fn ft_transfer_from(
        &mut self,
        sender_id: AccountId,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
    );
    fn ft_transfer(
        &mut self,
        receiver_id: AccountId,
        amount: U128,
        memo: Option<String>,
    );
    fn ft_balance_of(&self, account_id: AccountId) -> U128;
}

/// Event emitted when positions are split
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct PositionSplit {
    pub stakeholder: AccountId,
    pub collateral_token: AccountId,
    pub parent_collection_id: String,
    pub condition_id: String,
    pub partition: Vec<U128>,
    pub amount: U128,
}

/// Event emitted when positions are merged
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct PositionsMerge {
    pub stakeholder: AccountId,
    pub collateral_token: AccountId,
    pub parent_collection_id: String,
    pub condition_id: String,
    pub partition: Vec<U128>,
    pub amount: U128,
}

/// Event emitted when payouts are reported
#[derive(Serialize, Deserialize, Debug)]
#[serde(crate = "near_sdk::serde")]
pub struct PayoutRedemption {
    pub redeemer: AccountId,
    pub collateral_token: AccountId,
    pub parent_collection_id: String,
    pub condition_id: String,
    pub index_sets: Vec<Vec<U128>>,
    pub payout: U128,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct ConditionalTokenFramework {
    // Core state mappings following Gnosis CTF architecture
    
    /// Maps condition_id -> Condition
    pub conditions: UnorderedMap<String, Condition>,
    
    /// Maps collection_id -> Collection  
    pub collections: UnorderedMap<String, Collection>,
    
    /// Maps position_id -> Position
    pub positions: UnorderedMap<String, Position>,
    
    /// Maps "position_id:account_id" -> balance (ERC-1155 style)
    pub balances: UnorderedMap<String, U128>,
    
    /// Maps "owner:operator" -> approved (ERC-1155 style)
    pub operator_approvals: UnorderedMap<String, bool>,
    
    /// Maps "owner:position_id:operator" -> amount (ERC-1155 single token approval)
    pub token_approvals: UnorderedMap<String, U128>,
    
    /// Registered collateral tokens (USDC, etc.)
    pub collateral_tokens: UnorderedSet<AccountId>,
    
    /// Contract owner for administrative functions
    pub owner: AccountId,
}

#[near_bindgen]
impl ConditionalTokenFramework {
    #[init]
    pub fn new(owner: AccountId) -> Self {
        Self {
            conditions: UnorderedMap::new(b"c"),
            collections: UnorderedMap::new(b"o"),
            positions: UnorderedMap::new(b"p"),
            balances: UnorderedMap::new(b"b"),
            operator_approvals: UnorderedMap::new(b"a"),
            token_approvals: UnorderedMap::new(b"t"),
            collateral_tokens: UnorderedSet::new(b"k"),
            owner,
        }
    }

    // ============================================================================
    // CONDITION MANAGEMENT (following Gnosis CTF)
    // ============================================================================

    /// Prepare a condition for use in prediction markets
    /// This is equivalent to Gnosis CTF's prepareCondition
    pub fn prepare_condition(
        &mut self,
        oracle: AccountId,
        question_id: String,
        outcome_slot_count: u8,
    ) -> String {
        assert!(outcome_slot_count > 1, "Must have at least 2 outcomes");
        assert!(outcome_slot_count <= 255, "Too many outcomes");
        
        // Generate condition_id using same logic as Gnosis CTF
        let condition_id = self.get_condition_id(oracle.clone(), question_id.clone(), outcome_slot_count);
        
        // Check if condition already exists
        assert!(
            self.conditions.get(&condition_id).is_none(),
            "Condition already prepared"
        );
        
        let condition = Condition {
            oracle,
            question_id,
            outcome_slot_count,
            payout_numerators: None,
            payout_denominator: None,
        };
        
        self.conditions.insert(&condition_id, &condition);
        
        env::log_str(&format!(
            "ConditionPreparation: oracle={}, questionId={}, outcomeSlotCount={}, conditionId={}",
            condition.oracle, condition.question_id, condition.outcome_slot_count, condition_id
        ));
        
        condition_id
    }

    /// Report payouts for a condition (oracle only)
    /// This resolves the prediction market
    pub fn report_payouts(
        &mut self,
        question_id: String,
        payouts: Vec<U128>,
    ) {
        let caller = env::predecessor_account_id();
        
        // Find condition by question_id
        let mut condition_id = String::new();
        let mut found_condition: Option<Condition> = None;
        
        for (cid, condition) in self.conditions.iter() {
            if condition.question_id == question_id {
                assert_eq!(condition.oracle, caller, "Only oracle can report payouts");
                assert!(condition.payout_numerators.is_none(), "Payouts already reported");
                condition_id = cid;
                found_condition = Some(condition);
                break;
            }
        }
        
        let mut condition = found_condition.expect("Condition not found");
        
        assert_eq!(
            payouts.len() as u8,
            condition.outcome_slot_count,
            "Payout count must match outcome count"
        );
        
        // Calculate total payout for denominator
        let total_payout: u128 = payouts.iter().map(|p| p.0).sum();
        assert!(total_payout > 0, "Total payout must be positive");
        
        condition.payout_numerators = Some(payouts.clone());
        condition.payout_denominator = Some(U128(total_payout));
        
        self.conditions.insert(&condition_id, &condition);
        
        env::log_str(&format!(
            "PayoutRedemption: questionId={}, payouts={:?}, totalPayout={}",
            question_id, payouts, total_payout
        ));
    }

    /// Get condition by ID
    pub fn get_condition(&self, condition_id: String) -> Option<Condition> {
        self.conditions.get(&condition_id)
    }

    /// Check if condition is resolved
    pub fn is_condition_resolved(&self, condition_id: String) -> bool {
        if let Some(condition) = self.conditions.get(&condition_id) {
            condition.payout_numerators.is_some()
        } else {
            false
        }
    }

    // ============================================================================
    // POSITION SPLITTING AND MERGING (Core CTF Logic)
    // ============================================================================

    /// Split positions into outcome tokens
    /// This is the core CTF operation that converts collateral into conditional tokens
    pub fn split_position(
        &mut self,
        collateral_token: AccountId,
        parent_collection_id: String,
        condition_id: String,
        partition: Vec<U128>,
        amount: U128,
    ) {
        let caller = env::predecessor_account_id();
        
        // Validate inputs
        assert!(amount.0 > 0, "Amount must be positive");
        assert!(!partition.is_empty(), "Partition cannot be empty");
        
        // Verify condition exists
        let condition = self.conditions.get(&condition_id)
            .expect("Condition not found");
        
        // Validate partition matches condition outcomes
        let _full_index_set: Vec<U128> = (0..condition.outcome_slot_count)
            .map(|i| U128(1u128 << i))
            .collect();
        
        // Ensure partition covers all outcomes exactly once
        let mut covered_outcomes = 0u128;
        for index_set in &partition {
            assert!(index_set.0 != 0, "Empty index set not allowed");
            assert!(index_set.0 & covered_outcomes == 0, "Overlapping outcomes in partition");
            covered_outcomes |= index_set.0;
        }
        
        let expected_full_set = (1u128 << condition.outcome_slot_count) - 1;
        assert_eq!(covered_outcomes, expected_full_set, "Partition must cover all outcomes");
        
        // Get or create parent collection
        let parent_collection_key = if parent_collection_id.is_empty() {
            String::new()
        } else {
            parent_collection_id.clone()
        };
        
        // Check caller has sufficient balance of parent position
        if parent_collection_key.is_empty() {
            // Splitting from collateral token - transfer from caller
            self.transfer_collateral_from(caller.clone(), env::current_account_id(), collateral_token.clone(), amount);
        } else {
            // Splitting from parent position
            let parent_position_id = self.get_position_id(collateral_token.clone(), parent_collection_key.clone());
            let parent_balance_key = format!("{}:{}", parent_position_id, caller);
            let parent_balance = self.balances.get(&parent_balance_key).unwrap_or(U128(0));
            
            assert!(parent_balance.0 >= amount.0, "Insufficient parent position balance");
            
            // Burn parent position tokens
            self.balances.insert(&parent_balance_key, &U128(parent_balance.0 - amount.0));
        }
        
        // Create child positions and mint tokens
        for index_set in &partition {
            let collection_id = self.get_collection_id(parent_collection_key.clone(), condition_id.clone(), vec![*index_set]);
            let position_id = self.get_position_id(collateral_token.clone(), collection_id.clone());
            
            // Store collection if not exists
            if self.collections.get(&collection_id).is_none() {
                let collection = Collection {
                    collection_id: collection_id.clone(),
                    parent_collection_id: parent_collection_key.clone(),
                    condition_id: condition_id.clone(),
                    index_set: vec![*index_set],
                };
                self.collections.insert(&collection_id, &collection);
            }
            
            // Store position if not exists
            if self.positions.get(&position_id).is_none() {
                let position = Position {
                    position_id: position_id.clone(),
                    collateral_token: collateral_token.clone(),
                    collection_id: collection_id.clone(),
                    condition_id: condition_id.clone(),
                    index_set: vec![*index_set],
                };
                self.positions.insert(&position_id, &position);
            }
            
            // Mint tokens to caller
            let balance_key = format!("{}:{}", position_id, caller);
            let current_balance = self.balances.get(&balance_key).unwrap_or(U128(0));
            self.balances.insert(&balance_key, &U128(current_balance.0 + amount.0));
        }
        
        // Emit event
        let event = PositionSplit {
            stakeholder: caller,
            collateral_token,
            parent_collection_id: parent_collection_key,
            condition_id,
            partition,
            amount,
        };
        
        env::log_str(&format!("PositionSplit: {:?}", event));
    }

    /// Merge positions back into parent position or collateral
    /// This is the reverse of split_position
    pub fn merge_positions(
        &mut self,
        collateral_token: AccountId,
        parent_collection_id: String,
        condition_id: String,
        partition: Vec<U128>,
        amount: U128,
    ) {
        let caller = env::predecessor_account_id();
        
        // Validate inputs
        assert!(amount.0 > 0, "Amount must be positive");
        assert!(!partition.is_empty(), "Partition cannot be empty");
        
        // Verify condition exists
        let _condition = self.conditions.get(&condition_id)
            .expect("Condition not found");
        
        // Get parent collection
        let parent_collection_key = if parent_collection_id.is_empty() {
            String::new()
        } else {
            parent_collection_id.clone()
        };
        
        // Burn child position tokens
        for index_set in &partition {
            let collection_id = self.get_collection_id(parent_collection_key.clone(), condition_id.clone(), vec![*index_set]);
            let position_id = self.get_position_id(collateral_token.clone(), collection_id.clone());
            let balance_key = format!("{}:{}", position_id, caller);
            
            let balance = self.balances.get(&balance_key).unwrap_or(U128(0));
            assert!(balance.0 >= amount.0, "Insufficient balance for position merge");
            
            self.balances.insert(&balance_key, &U128(balance.0 - amount.0));
        }
        
        // Mint parent position or transfer collateral
        if parent_collection_key.is_empty() {
            // Merging to collateral token - transfer to caller
            self.transfer_collateral_to(env::current_account_id(), caller.clone(), collateral_token.clone(), amount);
        } else {
            // Merging to parent position
            let parent_position_id = self.get_position_id(collateral_token.clone(), parent_collection_key.clone());
            let parent_balance_key = format!("{}:{}", parent_position_id, caller);
            let parent_balance = self.balances.get(&parent_balance_key).unwrap_or(U128(0));
            
            self.balances.insert(&parent_balance_key, &U128(parent_balance.0 + amount.0));
        }
        
        // Emit event
        let event = PositionsMerge {
            stakeholder: caller,
            collateral_token,
            parent_collection_id: parent_collection_key,
            condition_id,
            partition,
            amount,
        };
        
        env::log_str(&format!("PositionsMerge: {:?}", event));
    }

    // ============================================================================
    // REDEMPTION AND PAYOUT SYSTEM (Polymarket Style)
    // ============================================================================

    /// Redeem positions for collateral after condition resolution
    /// This is the final step that converts winning outcome tokens back to collateral
    pub fn redeem_positions(
        &mut self,
        collateral_token: AccountId,
        parent_collection_id: String,
        condition_id: String,
        index_sets: Vec<Vec<U128>>,
    ) -> U128 {
        let caller = env::predecessor_account_id();
        
        // Verify condition is resolved
        let condition = self.conditions.get(&condition_id)
            .expect("Condition not found");
        
        let payout_numerators = condition.payout_numerators
            .as_ref()
            .expect("Condition not resolved yet");
        
        let payout_denominator = condition.payout_denominator
            .expect("Condition not resolved yet");
        
        let mut total_payout = 0u128;
        let parent_collection_key = if parent_collection_id.is_empty() {
            String::new()
        } else {
            parent_collection_id.clone()
        };
        
        // Process each index set (position type)
        for index_set in &index_sets {
            let collection_id = self.get_collection_id(parent_collection_key.clone(), condition_id.clone(), index_set.clone());
            let position_id = self.get_position_id(collateral_token.clone(), collection_id);
            let balance_key = format!("{}:{}", position_id, caller);
            
            // Get user's balance for this position
            let position_balance = self.balances.get(&balance_key).unwrap_or(U128(0));
            if position_balance.0 == 0 {
                continue; // Skip if user has no balance
            }
            
            // Calculate payout for this position
            let position_payout = self.calculate_position_payout(
                index_set,
                position_balance,
                payout_numerators,
                payout_denominator,
            );
            
            // Burn the position tokens
            self.balances.insert(&balance_key, &U128(0));
            
            total_payout += position_payout.0;
        }
        
        if total_payout > 0 {
            // Transfer collateral to user
            if parent_collection_key.is_empty() {
                // Redeeming for base collateral
                self.transfer_collateral_to(env::current_account_id(), caller.clone(), collateral_token.clone(), U128(total_payout));
            } else {
                // Redeeming for parent position
                let parent_position_id = self.get_position_id(collateral_token.clone(), parent_collection_key.clone());
                let parent_balance_key = format!("{}:{}", parent_position_id, caller);
                let parent_balance = self.balances.get(&parent_balance_key).unwrap_or(U128(0));
                
                self.balances.insert(&parent_balance_key, &U128(parent_balance.0 + total_payout));
            }
            
            // Emit redemption event
            let event = PayoutRedemption {
                redeemer: caller,
                collateral_token,
                parent_collection_id: parent_collection_key,
                condition_id,
                index_sets,
                payout: U128(total_payout),
            };
            
            env::log_str(&format!("PayoutRedemption: {:?}", event));
        }
        
        U128(total_payout)
    }

    /// Calculate payout for a specific position based on reported payouts
    fn calculate_position_payout(
        &self,
        index_set: &[U128],
        position_balance: U128,
        payout_numerators: &[U128],
        payout_denominator: U128,
    ) -> U128 {
        let mut total_payout_numerator = 0u128;
        
        // Sum payout numerators for all outcomes in this index set
        for &index in index_set {
            let mut outcome_index = 0;
            let mut temp_index = index.0;
            
            // Find which outcome this index represents
            while temp_index > 1 {
                temp_index >>= 1;
                outcome_index += 1;
            }
            
            if outcome_index < payout_numerators.len() {
                total_payout_numerator += payout_numerators[outcome_index].0;
            }
        }
        
        // Calculate proportional payout
        let payout = (position_balance.0 * total_payout_numerator) / payout_denominator.0;
        U128(payout)
    }

    /// Batch redeem multiple positions for gas efficiency
    pub fn batch_redeem_positions(
        &mut self,
        redemptions: Vec<(AccountId, String, String, Vec<Vec<U128>>)>,
    ) -> Vec<U128> {
        let mut results = Vec::new();
        
        for (collateral_token, parent_collection_id, condition_id, index_sets) in redemptions {
            let payout = self.redeem_positions(
                collateral_token,
                parent_collection_id,
                condition_id,
                index_sets,
            );
            results.push(payout);
        }
        
        results
    }

    // ============================================================================
    // ERC-1155 STYLE TOKEN OPERATIONS (Multi-Token Standard)
    // ============================================================================

    /// Set approval for all tokens for an operator (ERC-1155 style)
    pub fn set_approval_for_all(&mut self, operator: AccountId, approved: bool) {
        let owner = env::predecessor_account_id();
        let approval_key = format!("{}:{}", owner, operator);
        self.operator_approvals.insert(&approval_key, &approved);
        
        env::log_str(&format!(
            "ApprovalForAll: owner={} operator={} approved={}",
            owner, operator, approved
        ));
    }

    /// Check if operator is approved for all tokens of owner
    pub fn is_approved_for_all(&self, owner: AccountId, operator: AccountId) -> bool {
        let approval_key = format!("{}:{}", owner, operator);
        self.operator_approvals.get(&approval_key).unwrap_or(false)
    }

    /// Approve specific amount for a specific token (ERC-20 style for individual tokens)
    pub fn approve(&mut self, operator: AccountId, position_id: String, amount: U128) {
        let owner = env::predecessor_account_id();
        let approval_key = format!("{}:{}:{}", owner, position_id, operator);
        self.token_approvals.insert(&approval_key, &amount);
        
        env::log_str(&format!(
            "Approval: owner={} operator={} position_id={} amount={}",
            owner, operator, position_id, amount.0
        ));
    }

    /// Get allowance for specific token
    pub fn allowance(&self, owner: AccountId, operator: AccountId, position_id: String) -> U128 {
        let approval_key = format!("{}:{}:{}", owner, position_id, operator);
        self.token_approvals.get(&approval_key).unwrap_or(U128(0))
    }

    /// Safe transfer from one account to another (ERC-1155 style)
    pub fn safe_transfer_from(
        &mut self,
        from: AccountId,
        to: AccountId,
        position_id: String,
        amount: U128,
        data: Option<String>,
    ) {
        let caller = env::predecessor_account_id();
        
        // Check authorization
        assert!(
            caller == from || 
            self.is_approved_for_all(from.clone(), caller.clone()) ||
            self.allowance(from.clone(), caller.clone(), position_id.clone()).0 >= amount.0,
            "Transfer not authorized"
        );
        
        // Update specific token allowance if used
        if caller != from && !self.is_approved_for_all(from.clone(), caller.clone()) {
            let approval_key = format!("{}:{}:{}", from, position_id, caller);
            let current_allowance = self.token_approvals.get(&approval_key).unwrap_or(U128(0));
            assert!(current_allowance.0 >= amount.0, "Insufficient allowance");
            self.token_approvals.insert(&approval_key, &U128(current_allowance.0 - amount.0));
        }
        
        // Perform transfer
        self.transfer_position(from.clone(), to.clone(), position_id.clone(), amount);
        
        env::log_str(&format!(
            "TransferSingle: operator={} from={} to={} id={} value={}",
            caller, from, to, position_id, amount.0
        ));
        
        if let Some(data) = data {
            env::log_str(&format!("Transfer data: {}", data));
        }
    }

    /// Batch safe transfer multiple tokens (ERC-1155 style)
    pub fn safe_batch_transfer_from(
        &mut self,
        from: AccountId,
        to: AccountId,
        position_ids: Vec<String>,
        amounts: Vec<U128>,
        data: Option<String>,
    ) {
        let caller = env::predecessor_account_id();
        
        assert_eq!(position_ids.len(), amounts.len(), "Arrays length mismatch");
        
        // Check authorization (same as single transfer)
        let is_approved = caller == from || self.is_approved_for_all(from.clone(), caller.clone());
        
        for (i, position_id) in position_ids.iter().enumerate() {
            let amount = amounts[i];
            
            if !is_approved {
                let allowance = self.allowance(from.clone(), caller.clone(), position_id.clone());
                assert!(allowance.0 >= amount.0, "Insufficient allowance for batch transfer");
            }
        }
        
        // Perform transfers
        for (i, position_id) in position_ids.iter().enumerate() {
            let amount = amounts[i];
            
            // Update allowance if needed
            if !is_approved {
                let approval_key = format!("{}:{}:{}", from, position_id, caller);
                let current_allowance = self.token_approvals.get(&approval_key).unwrap_or(U128(0));
                self.token_approvals.insert(&approval_key, &U128(current_allowance.0 - amount.0));
            }
            
            self.transfer_position(from.clone(), to.clone(), position_id.clone(), amount);
        }
        
        env::log_str(&format!(
            "TransferBatch: operator={} from={} to={} ids={:?} values={:?}",
            caller, from, to, position_ids, amounts
        ));
        
        if let Some(data) = data {
            env::log_str(&format!("Batch transfer data: {}", data));
        }
    }

    /// Internal transfer function
    fn transfer_position(&mut self, from: AccountId, to: AccountId, position_id: String, amount: U128) {
        let from_key = format!("{}:{}", position_id, from);
        let to_key = format!("{}:{}", position_id, to);
        
        let from_balance = self.balances.get(&from_key).unwrap_or(U128(0));
        assert!(from_balance.0 >= amount.0, "Insufficient balance");
        
        self.balances.insert(&from_key, &U128(from_balance.0 - amount.0));
        
        let to_balance = self.balances.get(&to_key).unwrap_or(U128(0));
        self.balances.insert(&to_key, &U128(to_balance.0 + amount.0));
    }

    /// Get balance of a specific position for an account (ERC-1155 style)
    pub fn balance_of(&self, owner: AccountId, position_id: String) -> U128 {
        let balance_key = format!("{}:{}", position_id, owner);
        self.balances.get(&balance_key).unwrap_or(U128(0))
    }

    /// Get balances of multiple positions for multiple accounts (ERC-1155 style)
    pub fn balance_of_batch(&self, owners: Vec<AccountId>, position_ids: Vec<String>) -> Vec<U128> {
        assert_eq!(owners.len(), position_ids.len(), "Arrays length mismatch");
        
        let mut balances = Vec::new();
        for (i, owner) in owners.iter().enumerate() {
            let balance = self.balance_of(owner.clone(), position_ids[i].clone());
            balances.push(balance);
        }
        balances
    }

    // ============================================================================
    // UTILITY FUNCTIONS (ID Generation following Gnosis CTF)
    // ============================================================================

    /// Generate condition ID using same algorithm as Gnosis CTF
    /// conditionId = keccak256(abi.encodePacked(oracle, questionId, outcomeSlotCount))
    pub fn get_condition_id(&self, oracle: AccountId, question_id: String, outcome_slot_count: u8) -> String {
        let data = format!("{}:{}:{}", oracle, question_id, outcome_slot_count);
        let hash = sha256(data.as_bytes());
        hex::encode(hash)
    }

    /// Generate collection ID using same algorithm as Gnosis CTF
    /// collectionId = keccak256(abi.encodePacked(parentCollectionId, conditionId, indexSet))
    pub fn get_collection_id(&self, parent_collection_id: String, condition_id: String, index_set: Vec<U128>) -> String {
        let index_set_str: Vec<String> = index_set.iter().map(|i| i.0.to_string()).collect();
        let data = format!("{}:{}:{}", parent_collection_id, condition_id, index_set_str.join(","));
        let hash = sha256(data.as_bytes());
        hex::encode(hash)
    }

    /// Generate position ID using same algorithm as Gnosis CTF
    /// positionId = keccak256(abi.encodePacked(collateralToken, collectionId))
    pub fn get_position_id(&self, collateral_token: AccountId, collection_id: String) -> String {
        let data = format!("{}:{}", collateral_token, collection_id);
        let hash = sha256(data.as_bytes());
        hex::encode(hash)
    }

    /// Helper function to create index sets for binary outcomes
    /// Returns [1, 2] for YES/NO outcomes (bit positions)
    pub fn get_binary_index_sets(&self) -> Vec<U128> {
        vec![U128(1), U128(2)] // Bit 0 and Bit 1
    }

    /// Helper function to get full partition for condition
    pub fn get_full_partition(&self, outcome_slot_count: u8) -> Vec<U128> {
        (0..outcome_slot_count)
            .map(|i| U128(1u128 << i))
            .collect()
    }

    // ============================================================================
    // COLLATERAL TOKEN MANAGEMENT
    // ============================================================================

    /// Register a collateral token (admin only)
    pub fn register_collateral_token(&mut self, token: AccountId) {
        assert_eq!(env::predecessor_account_id(), self.owner, "Only owner can register tokens");
        self.collateral_tokens.insert(&token);
        env::log_str(&format!("Collateral token registered: {}", token));
    }

    /// Check if token is registered as collateral
    pub fn is_collateral_token_registered(&self, token: AccountId) -> bool {
        self.collateral_tokens.contains(&token)
    }

    /// Get all registered collateral tokens
    pub fn get_collateral_tokens(&self) -> Vec<AccountId> {
        self.collateral_tokens.to_vec()
    }

    /// Transfer collateral from user to contract (internal)
    fn transfer_collateral_from(&self, from: AccountId, to: AccountId, token: AccountId, amount: U128) {
        // In production, this would call the fungible token contract
        env::log_str(&format!(
            "CollateralTransferFrom: {} -> {} amount={} token={}",
            from, to, amount.0, token
        ));
        
        // ext_fungible_token::ext(token)
        //     .ft_transfer_from(from, to, amount, Some("CTF collateral transfer".to_string()));
    }

    /// Transfer collateral from contract to user (internal)
    fn transfer_collateral_to(&self, from: AccountId, to: AccountId, token: AccountId, amount: U128) {
        // In production, this would call the fungible token contract
        env::log_str(&format!(
            "CollateralTransferTo: {} -> {} amount={} token={}",
            from, to, amount.0, token
        ));
        
        // ext_fungible_token::ext(token)
        //     .ft_transfer(to, amount, Some("CTF payout transfer".to_string()));
    }

    // ============================================================================
    // QUERY FUNCTIONS
    // ============================================================================

    /// Get all conditions
    pub fn get_conditions(&self) -> Vec<(String, Condition)> {
        self.conditions.iter().collect()
    }

    /// Get all positions for a user
    pub fn get_user_positions(&self, user: AccountId) -> Vec<(String, U128)> {
        let mut positions = Vec::new();
        
        for (position_id, _) in self.positions.iter() {
            let balance = self.balance_of(user.clone(), position_id.clone());
            if balance.0 > 0 {
                positions.push((position_id, balance));
            }
        }
        
        positions
    }

    /// Get position details
    pub fn get_position(&self, position_id: String) -> Option<Position> {
        self.positions.get(&position_id)
    }

    /// Get collection details
    pub fn get_collection(&self, collection_id: String) -> Option<Collection> {
        self.collections.get(&collection_id)
    }

    /// Get total supply for a position
    pub fn total_supply(&self, position_id: String) -> U128 {
        let mut total = 0u128;
        
        // This is inefficient but works for demonstration
        // In production, you'd maintain a separate total supply mapping
        for (balance_key, balance) in self.balances.iter() {
            if balance_key.starts_with(&format!("{}:", position_id)) {
                total += balance.0;
            }
        }
        
        U128(total)
    }

    /// Check if position exists
    pub fn position_exists(&self, position_id: String) -> bool {
        self.positions.get(&position_id).is_some()
    }

    // ============================================================================
    // ADMIN FUNCTIONS
    // ============================================================================

    /// Update contract owner (current owner only)
    pub fn transfer_ownership(&mut self, new_owner: AccountId) {
        assert_eq!(env::predecessor_account_id(), self.owner, "Only current owner");
        let old_owner = self.owner.clone();
        self.owner = new_owner.clone();
        
        env::log_str(&format!(
            "OwnershipTransferred: {} -> {}",
            old_owner, new_owner
        ));
    }

    /// Get current owner
    pub fn get_owner(&self) -> AccountId {
        self.owner.clone()
    }

    /// Emergency pause (owner only) - placeholder for production safety
    pub fn emergency_pause(&mut self, paused: bool) {
        assert_eq!(env::predecessor_account_id(), self.owner, "Only owner can pause");
        env::log_str(&format!("Emergency pause: {}", paused));
        // In production, add paused state to contract
    }

    // ============================================================================
    // CONTRACT STATISTICS AND METADATA
    // ============================================================================

    /// Get contract statistics
    pub fn get_stats(&self) -> (u64, u64, u64, u64) {
        (
            self.conditions.len(),
            self.positions.len(),
            self.collections.len(),
            self.collateral_tokens.len(),
        )
    }

    /// Get contract version info
    pub fn get_version(&self) -> String {
        "ConditionalTokenFramework-NEAR-v1.0.0".to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use near_sdk::test_utils::VMContextBuilder;
    use near_sdk::{testing_env, VMContext};

    fn get_context(predecessor: &str) -> VMContext {
        VMContextBuilder::new()
            .predecessor_account_id(predecessor.parse().unwrap())
            .block_timestamp(1000000000000000000) // 1 second in nanoseconds
            .build()
    }

    #[test]
    fn test_prepare_condition() {
        testing_env!(get_context("oracle.testnet"));
        
        let mut contract = ConditionalTokenFramework::new("owner.testnet".parse().unwrap());
        
        // Register collateral token
        testing_env!(get_context("owner.testnet"));
        contract.register_collateral_token("usdc.testnet".parse().unwrap());
        
        // Prepare condition
        testing_env!(get_context("oracle.testnet"));
        let condition_id = contract.prepare_condition(
            "oracle.testnet".parse().unwrap(),
            "Will BTC reach $100k by 2025?".to_string(),
            2, // Binary outcome
        );
        
        // Verify condition exists
        let condition = contract.get_condition(condition_id.clone()).unwrap();
        assert_eq!(condition.oracle.as_str(), "oracle.testnet");
        assert_eq!(condition.question_id, "Will BTC reach $100k by 2025?");
        assert_eq!(condition.outcome_slot_count, 2);
        assert!(condition.payout_numerators.is_none());
        assert!(!contract.is_condition_resolved(condition_id));
    }

    #[test]
    fn test_split_position_basic() {
        testing_env!(get_context("user.testnet"));
        
        let mut contract = ConditionalTokenFramework::new("owner.testnet".parse().unwrap());
        
        // Register collateral and prepare condition
        testing_env!(get_context("owner.testnet"));
        contract.register_collateral_token("usdc.testnet".parse().unwrap());
        
        testing_env!(get_context("oracle.testnet"));
        let condition_id = contract.prepare_condition(
            "oracle.testnet".parse().unwrap(),
            "Test Market".to_string(),
            2,
        );
        
        // Split position
        testing_env!(get_context("user.testnet"));
        let partition = vec![U128(1), U128(2)]; // YES and NO outcomes
        
        contract.split_position(
            "usdc.testnet".parse().unwrap(),
            String::new(), // Empty parent collection (from collateral)
            condition_id.clone(),
            partition,
            U128(100_000_000), // 100 USDC
        );
        
        // Check that positions were created
        let collection_id_yes = contract.get_collection_id(String::new(), condition_id.clone(), vec![U128(1)]);
        let collection_id_no = contract.get_collection_id(String::new(), condition_id.clone(), vec![U128(2)]);
        
        let position_id_yes = contract.get_position_id("usdc.testnet".parse().unwrap(), collection_id_yes);
        let position_id_no = contract.get_position_id("usdc.testnet".parse().unwrap(), collection_id_no);
        
        // Check user balances
        let balance_yes = contract.balance_of("user.testnet".parse().unwrap(), position_id_yes);
        let balance_no = contract.balance_of("user.testnet".parse().unwrap(), position_id_no);
        
        assert_eq!(balance_yes.0, 100_000_000);
        assert_eq!(balance_no.0, 100_000_000);
    }

    #[test]
    fn test_merge_positions() {
        testing_env!(get_context("user.testnet"));
        
        let mut contract = ConditionalTokenFramework::new("owner.testnet".parse().unwrap());
        
        // Setup condition and split first
        testing_env!(get_context("owner.testnet"));
        contract.register_collateral_token("usdc.testnet".parse().unwrap());
        
        testing_env!(get_context("oracle.testnet"));
        let condition_id = contract.prepare_condition(
            "oracle.testnet".parse().unwrap(),
            "Test Market".to_string(),
            2,
        );
        
        testing_env!(get_context("user.testnet"));
        let partition = vec![U128(1), U128(2)];
        contract.split_position(
            "usdc.testnet".parse().unwrap(),
            String::new(),
            condition_id.clone(),
            partition.clone(),
            U128(100_000_000),
        );
        
        // Now merge back
        contract.merge_positions(
            "usdc.testnet".parse().unwrap(),
            String::new(),
            condition_id.clone(),
            partition,
            U128(50_000_000), // Merge half
        );
        
        // Check remaining balances
        let collection_id_yes = contract.get_collection_id(String::new(), condition_id.clone(), vec![U128(1)]);
        let position_id_yes = contract.get_position_id("usdc.testnet".parse().unwrap(), collection_id_yes);
        let balance_yes = contract.balance_of("user.testnet".parse().unwrap(), position_id_yes);
        
        assert_eq!(balance_yes.0, 50_000_000); // Original 100 - merged 50
    }

    #[test]
    fn test_report_payouts_and_redeem() {
        testing_env!(get_context("user.testnet"));
        
        let mut contract = ConditionalTokenFramework::new("owner.testnet".parse().unwrap());
        
        // Setup
        testing_env!(get_context("owner.testnet"));
        contract.register_collateral_token("usdc.testnet".parse().unwrap());
        
        testing_env!(get_context("oracle.testnet"));
        let condition_id = contract.prepare_condition(
            "oracle.testnet".parse().unwrap(),
            "Test Market".to_string(),
            2,
        );
        
        // Split position to get outcome tokens
        testing_env!(get_context("user.testnet"));
        let partition = vec![U128(1), U128(2)];
        contract.split_position(
            "usdc.testnet".parse().unwrap(),
            String::new(),
            condition_id.clone(),
            partition,
            U128(100_000_000),
        );
        
        // Oracle reports payouts (YES wins, NO loses)
        testing_env!(get_context("oracle.testnet"));
        contract.report_payouts(
            "Test Market".to_string(),
            vec![U128(100), U128(0)], // YES gets all, NO gets nothing
        );
        
        // User redeems YES tokens
        testing_env!(get_context("user.testnet"));
        let payout = contract.redeem_positions(
            "usdc.testnet".parse().unwrap(),
            String::new(),
            condition_id.clone(),
            vec![vec![U128(1)]], // Redeem YES tokens
        );
        
        assert_eq!(payout.0, 100_000_000); // Full payout for winning outcome
        
        // Check that YES tokens were burned
        let collection_id_yes = contract.get_collection_id(String::new(), condition_id.clone(), vec![U128(1)]);
        let position_id_yes = contract.get_position_id("usdc.testnet".parse().unwrap(), collection_id_yes);
        let balance_yes = contract.balance_of("user.testnet".parse().unwrap(), position_id_yes);
        
        assert_eq!(balance_yes.0, 0); // Tokens burned during redemption
    }

    #[test]
    fn test_erc1155_transfers() {
        testing_env!(get_context("user.testnet"));
        
        let mut contract = ConditionalTokenFramework::new("owner.testnet".parse().unwrap());
        
        // Setup condition and positions
        testing_env!(get_context("owner.testnet"));
        contract.register_collateral_token("usdc.testnet".parse().unwrap());
        
        testing_env!(get_context("oracle.testnet"));
        let condition_id = contract.prepare_condition(
            "oracle.testnet".parse().unwrap(),
            "Test Market".to_string(),
            2,
        );
        
        testing_env!(get_context("user.testnet"));
        contract.split_position(
            "usdc.testnet".parse().unwrap(),
            String::new(),
            condition_id.clone(),
            vec![U128(1), U128(2)],
            U128(100_000_000),
        );
        
        // Get position ID
        let collection_id_yes = contract.get_collection_id(String::new(), condition_id.clone(), vec![U128(1)]);
        let position_id_yes = contract.get_position_id("usdc.testnet".parse().unwrap(), collection_id_yes);
        
        // Transfer tokens to another user
        contract.safe_transfer_from(
            "user.testnet".parse().unwrap(),
            "receiver.testnet".parse().unwrap(),
            position_id_yes.clone(),
            U128(25_000_000),
            None,
        );
        
        // Check balances
        let sender_balance = contract.balance_of("user.testnet".parse().unwrap(), position_id_yes.clone());
        let receiver_balance = contract.balance_of("receiver.testnet".parse().unwrap(), position_id_yes.clone());
        
        assert_eq!(sender_balance.0, 75_000_000);
        assert_eq!(receiver_balance.0, 25_000_000);
    }

    #[test]
    fn test_approval_system() {
        testing_env!(get_context("user.testnet"));
        
        let mut contract = ConditionalTokenFramework::new("owner.testnet".parse().unwrap());
        
        // Test approval for all
        contract.set_approval_for_all("operator.testnet".parse().unwrap(), true);
        
        assert!(contract.is_approved_for_all(
            "user.testnet".parse().unwrap(),
            "operator.testnet".parse().unwrap()
        ));
        
        // Setup a position for specific token approval
        testing_env!(get_context("owner.testnet"));
        contract.register_collateral_token("usdc.testnet".parse().unwrap());
        
        testing_env!(get_context("oracle.testnet"));
        let condition_id = contract.prepare_condition(
            "oracle.testnet".parse().unwrap(),
            "Test Market".to_string(),
            2,
        );
        
        testing_env!(get_context("user.testnet"));
        contract.split_position(
            "usdc.testnet".parse().unwrap(),
            String::new(),
            condition_id.clone(),
            vec![U128(1), U128(2)],
            U128(100_000_000),
        );
        
        let collection_id = contract.get_collection_id(String::new(), condition_id.clone(), vec![U128(1)]);
        let position_id = contract.get_position_id("usdc.testnet".parse().unwrap(), collection_id);
        
        // Test specific token approval
        contract.approve(
            "spender.testnet".parse().unwrap(),
            position_id.clone(),
            U128(50_000_000),
        );
        
        let allowance = contract.allowance(
            "user.testnet".parse().unwrap(),
            "spender.testnet".parse().unwrap(),
            position_id,
        );
        
        assert_eq!(allowance.0, 50_000_000);
    }

    #[test]
    fn test_id_generation() {
        let contract = ConditionalTokenFramework::new("owner.testnet".parse().unwrap());
        
        // Test condition ID generation
        let condition_id1 = contract.get_condition_id(
            "oracle.testnet".parse().unwrap(),
            "Question 1".to_string(),
            2,
        );
        
        let condition_id2 = contract.get_condition_id(
            "oracle.testnet".parse().unwrap(),
            "Question 2".to_string(),
            2,
        );
        
        // Should be different for different questions
        assert_ne!(condition_id1, condition_id2);
        
        // Should be consistent for same inputs
        let condition_id1_dup = contract.get_condition_id(
            "oracle.testnet".parse().unwrap(),
            "Question 1".to_string(),
            2,
        );
        assert_eq!(condition_id1, condition_id1_dup);
        
        // Test collection ID generation
        let collection_id1 = contract.get_collection_id(
            String::new(),
            condition_id1.clone(),
            vec![U128(1)],
        );
        
        let collection_id2 = contract.get_collection_id(
            String::new(),
            condition_id1.clone(),
            vec![U128(2)],
        );
        
        assert_ne!(collection_id1, collection_id2);
        
        // Test position ID generation
        let position_id1 = contract.get_position_id(
            "usdc.testnet".parse().unwrap(),
            collection_id1.clone(),
        );
        
        let position_id2 = contract.get_position_id(
            "dai.testnet".parse().unwrap(),
            collection_id1.clone(),
        );
        
        assert_ne!(position_id1, position_id2);
    }

    #[test]
    fn test_batch_operations() {
        testing_env!(get_context("user.testnet"));
        
        let mut contract = ConditionalTokenFramework::new("owner.testnet".parse().unwrap());
        
        // Setup multiple conditions and positions
        testing_env!(get_context("owner.testnet"));
        contract.register_collateral_token("usdc.testnet".parse().unwrap());
        
        testing_env!(get_context("oracle.testnet"));
        let condition_id1 = contract.prepare_condition(
            "oracle.testnet".parse().unwrap(),
            "Market 1".to_string(),
            2,
        );
        
        let condition_id2 = contract.prepare_condition(
            "oracle.testnet".parse().unwrap(),
            "Market 2".to_string(),
            2,
        );
        
        // Create positions
        testing_env!(get_context("user.testnet"));
        contract.split_position(
            "usdc.testnet".parse().unwrap(),
            String::new(),
            condition_id1.clone(),
            vec![U128(1), U128(2)],
            U128(100_000_000),
        );
        
        contract.split_position(
            "usdc.testnet".parse().unwrap(),
            String::new(),
            condition_id2.clone(),
            vec![U128(1), U128(2)],
            U128(100_000_000),
        );
        
        // Get position IDs
        let collection_id1 = contract.get_collection_id(String::new(), condition_id1.clone(), vec![U128(1)]);
        let collection_id2 = contract.get_collection_id(String::new(), condition_id2.clone(), vec![U128(1)]);
        
        let position_id1 = contract.get_position_id("usdc.testnet".parse().unwrap(), collection_id1);
        let position_id2 = contract.get_position_id("usdc.testnet".parse().unwrap(), collection_id2);
        
        // Test batch balance query
        let balances = contract.balance_of_batch(
            vec![
                "user.testnet".parse().unwrap(),
                "user.testnet".parse().unwrap(),
            ],
            vec![position_id1.clone(), position_id2.clone()],
        );
        
        assert_eq!(balances[0].0, 100_000_000);
        assert_eq!(balances[1].0, 100_000_000);
        
        // Test batch transfer
        contract.safe_batch_transfer_from(
            "user.testnet".parse().unwrap(),
            "receiver.testnet".parse().unwrap(),
            vec![position_id1.clone(), position_id2.clone()],
            vec![U128(25_000_000), U128(50_000_000)],
            None,
        );
        
        // Check updated balances
        let updated_balances = contract.balance_of_batch(
            vec![
                "user.testnet".parse().unwrap(),
                "receiver.testnet".parse().unwrap(),
            ],
            vec![position_id1, position_id2],
        );
        
        assert_eq!(updated_balances[0].0, 75_000_000); // user remaining for position1
        assert_eq!(updated_balances[1].0, 50_000_000); // receiver received for position2
    }

    #[test]
    fn test_contract_stats_and_queries() {
        testing_env!(get_context("owner.testnet"));
        
        let mut contract = ConditionalTokenFramework::new("owner.testnet".parse().unwrap());
        
        // Initial stats
        let (conditions, positions, collections, tokens) = contract.get_stats();
        assert_eq!(conditions, 0);
        assert_eq!(positions, 0);
        assert_eq!(collections, 0);
        assert_eq!(tokens, 0);
        
        // Register token and prepare condition
        contract.register_collateral_token("usdc.testnet".parse().unwrap());
        
        testing_env!(get_context("oracle.testnet"));
        let condition_id = contract.prepare_condition(
            "oracle.testnet".parse().unwrap(),
            "Test Market".to_string(),
            2,
        );
        
        // Create positions
        testing_env!(get_context("user.testnet"));
        contract.split_position(
            "usdc.testnet".parse().unwrap(),
            String::new(),
            condition_id.clone(),
            vec![U128(1), U128(2)],
            U128(100_000_000),
        );
        
        // Check updated stats
        let (conditions, positions, collections, tokens) = contract.get_stats();
        assert_eq!(conditions, 1);
        assert_eq!(positions, 2); // YES and NO positions
        assert_eq!(collections, 2); // YES and NO collections
        assert_eq!(tokens, 1); // USDC
        
        // Test user positions query
        let user_positions = contract.get_user_positions("user.testnet".parse().unwrap());
        assert_eq!(user_positions.len(), 2); // User has 2 positions
        
        // Test version info
        let version = contract.get_version();
        assert!(version.contains("ConditionalTokenFramework-NEAR"));
    }
}