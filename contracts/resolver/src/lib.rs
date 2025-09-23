use near_sdk::borsh::{self, BorshDeserialize, BorshSerialize};
use near_sdk::collections::{UnorderedMap, UnorderedSet};
use near_sdk::json_types::U128;
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, near_bindgen, AccountId, Promise, PanicOnDefault};
use schemars::JsonSchema;

// Local type definitions for standalone contract
#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Condition {
    pub condition_id: String,
    pub market_id: String,
    pub outcome_slot_count: u8,
    #[schemars(with = "String")]
    pub oracle: AccountId,
    pub question_id: String,
    pub is_resolved: bool,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Market {
    pub market_id: String,
    pub condition_id: String,
    pub title: String,
    pub description: String,
    #[schemars(with = "String")]
    pub creator: AccountId,
    pub end_time: u64,
    pub resolution_time: u64,
    pub category: String,
    pub is_active: bool,
    #[schemars(with = "String")]
    pub resolver: AccountId,
}

// External contract interfaces
#[near_sdk::ext_contract(ext_ctf)]
pub trait ConditionalTokenFramework {
    fn report_payout_numerators(&mut self, condition_id: String, payout_numerators: Vec<U128>);
    fn get_condition(&self, condition_id: String) -> Option<Condition>;
}

#[near_sdk::ext_contract(ext_verifier)]
pub trait PredictionVerifier {
    fn get_market(&self, market_id: String) -> Option<Market>;
}

#[near_sdk::ext_contract(ext_self)]
pub trait ResolverCallbacks {
    fn on_market_info_for_resolution(
        &mut self, 
        market_id: String, 
        winning_outcome: u8,
        #[callback_result] market_result: Result<Option<Market>, near_sdk::PromiseError>
    ) -> Promise;
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Resolution {
    pub market_id: String,
    pub condition_id: String,
    #[schemars(with = "String")]
    pub resolver: AccountId,
    pub winning_outcome: u8,                                       // 0=NO, 1=YES, 2=INVALID
    pub resolution_data: String,                                   // JSON with evidence/reasoning
    pub submitted_at: u64,
    pub finalized_at: Option<u64>,
    pub status: ResolutionStatus,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum ResolutionStatus {
    Pending,        // Submitted but in dispute period
    Disputed,       // Dispute raised, needs review
    Finalized,      // Resolution final, payouts enabled
    Invalid,        // Market declared invalid, full refunds
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema, Clone)]
#[serde(crate = "near_sdk::serde")]
pub struct Dispute {
    pub market_id: String,
    #[schemars(with = "String")]
    pub disputer: AccountId,
    pub reason: String,
    pub evidence: String,
    #[schemars(with = "String")]
    pub bond_amount: U128,
    pub created_at: u64,
    pub resolved_at: Option<u64>,
    pub dispute_outcome: Option<DisputeOutcome>,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, JsonSchema, Clone)]
#[serde(crate = "near_sdk::serde")]
pub enum DisputeOutcome {
    DisputeWins,    // Original resolution overturned
    DisputeLoses,   // Original resolution stands
    MarketInvalid,  // Market declared invalid
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct MarketResolver {
    pub owner_id: AccountId,
    pub verifier_contract: AccountId,                              // PredictionVerifier address
    pub ctf_contract: AccountId,                                   // ConditionalTokenFramework address
    pub authorized_oracles: UnorderedSet<AccountId>,               // Who can submit resolutions
    pub resolutions: UnorderedMap<String, Resolution>,             // market_id -> Resolution
    pub disputes: UnorderedMap<String, Dispute>,                   // market_id -> Dispute
    pub dispute_period: u64,                                       // Time window for disputes (nanoseconds)
    pub dispute_bond: U128,                                        // NEAR required to start dispute
}

#[near_bindgen]
impl MarketResolver {
    #[init]
    pub fn new(
        owner_id: AccountId,
        verifier_contract: AccountId,
        ctf_contract: AccountId,
        dispute_period: u64,
        dispute_bond: U128,
    ) -> Self {
        Self {
            owner_id,
            verifier_contract,
            ctf_contract,
            authorized_oracles: UnorderedSet::new(b"o"),
            resolutions: UnorderedMap::new(b"r"),
            disputes: UnorderedMap::new(b"d"),
            dispute_period,
            dispute_bond,
        }
    }

    // Resolution Management
    pub fn submit_resolution(
        &mut self,
        market_id: String,
        winning_outcome: u8,
        resolution_data: String,
    ) -> String {
        let caller = env::predecessor_account_id();
        
        // Check authorization
        assert!(
            self.authorized_oracles.contains(&caller) || caller == self.owner_id,
            "Not authorized to submit resolutions"
        );

        // Validate outcome (0=NO, 1=YES, 2=INVALID)
        assert!(winning_outcome <= 2, "Invalid outcome value");

        // Check if already resolved
        assert!(
            self.resolutions.get(&market_id).is_none(),
            "Market already has a resolution"
        );

        // Get market info to validate timing
        // In production, this would be a cross-contract call
        // For now, we'll assume the resolver can submit after resolution_time

        let resolution_id = format!("resolution_{}_{}", market_id, env::block_timestamp());
        
        let resolution = Resolution {
            market_id: market_id.clone(),
            condition_id: String::new(), // Will be filled from market data
            resolver: caller.clone(),
            winning_outcome,
            resolution_data,
            submitted_at: env::block_timestamp(),
            finalized_at: None,
            status: ResolutionStatus::Pending,
        };

        self.resolutions.insert(&market_id, &resolution);

        env::log_str(&format!(
            "Resolution submitted for market {}: outcome {} by {}",
            market_id, winning_outcome, caller
        ));

        resolution_id
    }

    // Finalize resolution after dispute period
    pub fn finalize_resolution(&mut self, market_id: String) -> Promise {
        let mut resolution = self.resolutions.get(&market_id)
            .expect("Resolution not found");

        // Check if dispute period has passed
        let dispute_deadline = resolution.submitted_at + self.dispute_period;
        assert!(
            env::block_timestamp() > dispute_deadline,
            "Dispute period has not ended"
        );

        // Check if there's an active dispute
        if let Some(dispute) = self.disputes.get(&market_id) {
            assert!(
                dispute.resolved_at.is_some(),
                "Cannot finalize while dispute is active"
            );
        }

        // Update resolution status
        resolution.status = ResolutionStatus::Finalized;
        resolution.finalized_at = Some(env::block_timestamp());
        self.resolutions.insert(&market_id, &resolution);

        env::log_str(&format!("Resolution finalized for market {}", market_id));

        // Get condition_id from verifier contract first, then set payout numerators
        ext_verifier::ext(self.verifier_contract.clone())
            .with_static_gas(near_sdk::Gas::from_tgas(5))
            .get_market(market_id.clone())
            .then(
                ext_self::ext(env::current_account_id())
                    .with_static_gas(near_sdk::Gas::from_tgas(10))
                    .on_market_info_for_resolution(market_id, resolution.winning_outcome)
            )
    }

    // Check if market is resolved and finalized
    pub fn is_market_finalized(&self, market_id: String) -> bool {
        if let Some(resolution) = self.resolutions.get(&market_id) {
            matches!(resolution.status, ResolutionStatus::Finalized)
        } else {
            false
        }
    }

    // Dispute Mechanism
    #[payable]
    pub fn dispute_resolution(
        &mut self,
        market_id: String,
        reason: String,
        evidence: String,
    ) -> String {
        let resolution = self.resolutions.get(&market_id)
            .expect("Resolution not found");

        // Check if resolution is in dispute period
        let dispute_deadline = resolution.submitted_at + self.dispute_period;
        assert!(
            env::block_timestamp() <= dispute_deadline,
            "Dispute period has ended"
        );

        // Check if already disputed
        assert!(
            self.disputes.get(&market_id).is_none(),
            "Market already disputed"
        );

        // Check bond amount
        let attached_deposit = env::attached_deposit();
        assert!(
            attached_deposit.as_yoctonear() >= self.dispute_bond.0,
            "Insufficient dispute bond"
        );

        let caller = env::predecessor_account_id();
        let dispute_id = format!("dispute_{}_{}", market_id, env::block_timestamp());

        let dispute = Dispute {
            market_id: market_id.clone(),
            disputer: caller.clone(),
            reason,
            evidence,
            bond_amount: U128(attached_deposit.as_yoctonear()),
            created_at: env::block_timestamp(),
            resolved_at: None,
            dispute_outcome: None,
        };

        self.disputes.insert(&market_id, &dispute);

        // Update resolution status
        let mut resolution = self.resolutions.get(&market_id).unwrap();
        resolution.status = ResolutionStatus::Disputed;
        self.resolutions.insert(&market_id, &resolution);

        env::log_str(&format!(
            "Dispute raised for market {} by {} with {} NEAR bond",
            market_id, caller, attached_deposit
        ));

        dispute_id
    }

    // Resolve dispute (admin function)
    pub fn resolve_dispute(
        &mut self,
        market_id: String,
        outcome: DisputeOutcome,
        explanation: String,
    ) -> Promise {
        assert_eq!(env::predecessor_account_id(), self.owner_id, "Only owner can resolve disputes");

        let mut dispute = self.disputes.get(&market_id)
            .expect("Dispute not found");
        
        assert!(dispute.resolved_at.is_none(), "Dispute already resolved");

        dispute.resolved_at = Some(env::block_timestamp());
        dispute.dispute_outcome = Some(outcome.clone());
        self.disputes.insert(&market_id, &dispute);

        let mut resolution = self.resolutions.get(&market_id).unwrap();

        match outcome {
            DisputeOutcome::DisputeWins => {
                // Disputer wins - need to update resolution or invalidate market
                resolution.status = ResolutionStatus::Invalid;
                self.resolutions.insert(&market_id, &resolution);
                
                env::log_str(&format!("Dispute won for market {}: {}", market_id, explanation));
                
                // Return bond to disputer
                Promise::new(dispute.disputer).transfer(near_sdk::NearToken::from_yoctonear(dispute.bond_amount.0))
            }
            DisputeOutcome::DisputeLoses => {
                // Original resolution stands
                resolution.status = ResolutionStatus::Pending;
                self.resolutions.insert(&market_id, &resolution);
                
                env::log_str(&format!("Dispute lost for market {}: {}", market_id, explanation));
                
                // Keep dispute bond (could be used for platform treasury)
                Promise::new(env::current_account_id())
            }
            DisputeOutcome::MarketInvalid => {
                // Market declared invalid
                resolution.status = ResolutionStatus::Invalid;
                resolution.winning_outcome = 2; // INVALID
                self.resolutions.insert(&market_id, &resolution);
                
                env::log_str(&format!("Market {} declared invalid: {}", market_id, explanation));
                
                // Return bond to disputer
                Promise::new(dispute.disputer).transfer(near_sdk::NearToken::from_yoctonear(dispute.bond_amount.0))
            }
        }
    }

    // Oracle Management
    pub fn add_oracle(&mut self, oracle: AccountId) {
        assert_eq!(env::predecessor_account_id(), self.owner_id, "Only owner can add oracles");
        self.authorized_oracles.insert(&oracle);
        env::log_str(&format!("Oracle {} added", oracle));
    }

    pub fn remove_oracle(&mut self, oracle: AccountId) {
        assert_eq!(env::predecessor_account_id(), self.owner_id, "Only owner can remove oracles");
        self.authorized_oracles.remove(&oracle);
        env::log_str(&format!("Oracle {} removed", oracle));
    }

    pub fn is_authorized_oracle(&self, oracle: AccountId) -> bool {
        self.authorized_oracles.contains(&oracle)
    }

    // Payout Distribution
    fn set_payout_numerators(&self, condition_id: String, winning_outcome: u8) -> Promise {
        let payout_numerators = match winning_outcome {
            0 => vec![U128(1_000_000_000_000_000_000_000_000), U128(0)], // NO wins
            1 => vec![U128(0), U128(1_000_000_000_000_000_000_000_000)], // YES wins
            2 => vec![U128(500_000_000_000_000_000_000_000), U128(500_000_000_000_000_000_000_000)], // INVALID - 50/50 split
            _ => panic!("Invalid winning outcome"),
        };

        env::log_str(&format!(
            "Setting payout numerators for condition {}: [{}, {}]",
            condition_id, payout_numerators[0].0, payout_numerators[1].0
        ));

        ext_ctf::ext(self.ctf_contract.clone())
            .report_payout_numerators(condition_id, payout_numerators)
    }

    // Handle invalid market (full refunds)
    fn handle_invalid_market(&self, condition_id: String) -> Promise {
        // Set equal payouts for both outcomes (50/50 split)
        self.set_payout_numerators(condition_id, 2)
    }

    // View Methods
    pub fn get_resolution(&self, market_id: String) -> Option<Resolution> {
        self.resolutions.get(&market_id)
    }

    pub fn get_dispute(&self, market_id: String) -> Option<Dispute> {
        self.disputes.get(&market_id)
    }

    pub fn get_authorized_oracles(&self) -> Vec<AccountId> {
        self.authorized_oracles.to_vec()
    }

    pub fn get_dispute_config(&self) -> (u64, U128) {
        (self.dispute_period, self.dispute_bond)
    }

    pub fn get_pending_resolutions(&self) -> Vec<Resolution> {
        let mut pending = Vec::new();
        for (_, resolution) in self.resolutions.iter() {
            if matches!(resolution.status, ResolutionStatus::Pending) {
                pending.push(resolution);
            }
        }
        pending
    }

    pub fn get_disputed_resolutions(&self) -> Vec<Resolution> {
        let mut disputed = Vec::new();
        for (_, resolution) in self.resolutions.iter() {
            if matches!(resolution.status, ResolutionStatus::Disputed) {
                disputed.push(resolution);
            }
        }
        disputed
    }

    // Configuration
    pub fn update_dispute_period(&mut self, new_period: u64) {
        assert_eq!(env::predecessor_account_id(), self.owner_id, "Only owner can update dispute period");
        
        // Minimum 1 hour, maximum 7 days
        assert!(new_period >= 3_600_000_000_000, "Dispute period too short (min 1 hour)");
        assert!(new_period <= 604_800_000_000_000, "Dispute period too long (max 7 days)");
        
        self.dispute_period = new_period;
        env::log_str(&format!("Dispute period updated to {} nanoseconds", new_period));
    }

    pub fn update_dispute_bond(&mut self, new_bond: U128) {
        assert_eq!(env::predecessor_account_id(), self.owner_id, "Only owner can update dispute bond");
        
        // Minimum 1 NEAR
        assert!(new_bond.0 >= 1_000_000_000_000_000_000_000_000, "Dispute bond too low (min 1 NEAR)");
        
        self.dispute_bond = new_bond;
        env::log_str(&format!("Dispute bond updated to {} yoctoNEAR", new_bond.0));
    }

    // Callback to handle market info and set payout numerators
    #[private]
    pub fn on_market_info_for_resolution(
        &mut self, 
        market_id: String, 
        winning_outcome: u8,
        #[callback_result] market_result: Result<Option<Market>, near_sdk::PromiseError>
    ) -> Promise {
        match market_result {
            Ok(Some(market)) => {
                env::log_str(&format!(
                    "Setting payout numerators for market {} with condition {}", 
                    market_id, market.condition_id
                ));
                
                // Now we have the real condition_id from the market
                self.set_payout_numerators(market.condition_id, winning_outcome)
            }
            Ok(None) => {
                env::log_str(&format!("Market {} not found during resolution", market_id));
                Promise::new(env::current_account_id())
            }
            Err(e) => {
                env::log_str(&format!("Failed to get market info for {}: {:?}", market_id, e));
                Promise::new(env::current_account_id())
            }
        }
    }

    // Emergency functions
    pub fn emergency_resolve(&mut self, market_id: String, winning_outcome: u8) -> Promise {
        assert_eq!(env::predecessor_account_id(), self.owner_id, "Only owner can emergency resolve");
        
        let mut resolution = self.resolutions.get(&market_id)
            .expect("Resolution not found");
        
        resolution.winning_outcome = winning_outcome;
        resolution.status = ResolutionStatus::Finalized;
        resolution.finalized_at = Some(env::block_timestamp());
        self.resolutions.insert(&market_id, &resolution);

        env::log_str(&format!("Emergency resolution for market {}: outcome {}", market_id, winning_outcome));

        // Get market info first for condition_id
        ext_verifier::ext(self.verifier_contract.clone())
            .with_static_gas(near_sdk::Gas::from_tgas(5))
            .get_market(market_id.clone())
            .then(
                ext_self::ext(env::current_account_id())
                    .with_static_gas(near_sdk::Gas::from_tgas(10))
                    .on_market_info_for_resolution(market_id, winning_outcome)
            )
    }
}
