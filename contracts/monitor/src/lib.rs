use near_sdk::borsh::{BorshDeserialize, BorshSerialize};
use near_sdk::collections::{UnorderedMap, UnorderedSet};
use near_sdk::serde::{Deserialize, Serialize};
use near_sdk::{env, near_bindgen, AccountId, PanicOnDefault, Promise};
use serde_json;

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
pub struct BridgeTransaction {
    pub tx_hash: String,
    pub source_chain: u32,
    pub target_chain: u32,
    pub user: AccountId,
    pub amount: String,
    pub token: String,
    pub status: TransactionStatus,
    pub created_at: u64,
    pub updated_at: u64,
    pub retry_count: u8,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
pub struct FailedTransaction {
    pub tx_hash: String,
    pub error_message: String,
    pub failed_at: u64,
    pub recovery_action: Option<RecoveryAction>,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
pub struct ProgressTracker {
    pub tx_hash: String,
    pub current_step: BridgeStep,
    pub total_steps: u8,
    pub estimated_completion: u64,
    pub last_update: u64,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
pub enum TransactionStatus {
    Initiated,
    SourceConfirmed,
    BridgeProcessing,
    TargetPending,
    Completed,
    Failed,
    RequiresAttention,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
pub enum BridgeStep {
    InitiateTransaction,
    WaitSourceConfirmation,
    ProcessBridge,
    WaitTargetConfirmation,
    Complete,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
pub enum RecoveryAction {
    Retry,
    ManualIntervention,
    RefundToSource,
    CompleteManually,
}

#[derive(BorshDeserialize, BorshSerialize, Serialize, Deserialize, Clone, Debug)]
pub struct AlertThresholds {
    pub max_processing_time: u64,
    pub max_retry_count: u8,
    pub stuck_transaction_threshold: u64,
}

#[near_bindgen]
#[derive(BorshDeserialize, BorshSerialize, PanicOnDefault)]
pub struct CrossChainMonitor {
    pub owner_id: AccountId,
    pub bridge_transactions: UnorderedMap<String, BridgeTransaction>,
    pub failed_transactions: UnorderedMap<String, FailedTransaction>,
    pub progress_tracking: UnorderedMap<String, ProgressTracker>,
    pub retry_queue: UnorderedSet<String>,
    pub alert_thresholds: AlertThresholds,
    pub monitoring_enabled: bool,
}

#[near_bindgen]
impl CrossChainMonitor {
    #[init]
    pub fn new(owner_id: AccountId) -> Self {
        Self {
            owner_id,
            bridge_transactions: UnorderedMap::new(b"b"),
            failed_transactions: UnorderedMap::new(b"f"),
            progress_tracking: UnorderedMap::new(b"p"),
            retry_queue: UnorderedSet::new(b"r"),
            alert_thresholds: AlertThresholds {
                max_processing_time: 3600000000000, // 1 hour in nanoseconds
                max_retry_count: 5,
                stuck_transaction_threshold: 7200000000000, // 2 hours
            },
            monitoring_enabled: true,
        }
    }

    pub fn start_bridge_transaction(
        &mut self,
        tx_hash: String,
        source_chain: u32,
        target_chain: u32,
        user: AccountId,
        amount: String,
        token: String,
    ) {
        let transaction = BridgeTransaction {
            tx_hash: tx_hash.clone(),
            source_chain,
            target_chain,
            user,
            amount,
            token,
            status: TransactionStatus::Initiated,
            created_at: env::block_timestamp(),
            updated_at: env::block_timestamp(),
            retry_count: 0,
        };

        let progress = ProgressTracker {
            tx_hash: tx_hash.clone(),
            current_step: BridgeStep::InitiateTransaction,
            total_steps: 5,
            estimated_completion: env::block_timestamp() + self.alert_thresholds.max_processing_time,
            last_update: env::block_timestamp(),
        };

        self.bridge_transactions.insert(&tx_hash, &transaction);
        self.progress_tracking.insert(&tx_hash, &progress);
    }

    pub fn update_transaction_status(&mut self, tx_hash: String, status: TransactionStatus) {
        if let Some(mut transaction) = self.bridge_transactions.get(&tx_hash) {
            transaction.status = status;
            transaction.updated_at = env::block_timestamp();
            self.bridge_transactions.insert(&tx_hash, &transaction);

            if let Some(mut progress) = self.progress_tracking.get(&tx_hash) {
                progress.current_step = match transaction.status {
                    TransactionStatus::Initiated => BridgeStep::InitiateTransaction,
                    TransactionStatus::SourceConfirmed => BridgeStep::WaitSourceConfirmation,
                    TransactionStatus::BridgeProcessing => BridgeStep::ProcessBridge,
                    TransactionStatus::TargetPending => BridgeStep::WaitTargetConfirmation,
                    TransactionStatus::Completed => BridgeStep::Complete,
                    _ => progress.current_step,
                };
                progress.last_update = env::block_timestamp();
                self.progress_tracking.insert(&tx_hash, &progress);
            }
        }
    }

    pub fn get_bridge_status(&self, tx_hash: String) -> Option<BridgeTransaction> {
        self.bridge_transactions.get(&tx_hash)
    }

    pub fn get_progress(&self, tx_hash: String) -> Option<ProgressTracker> {
        self.progress_tracking.get(&tx_hash)
    }

    pub fn mark_transaction_failed(&mut self, tx_hash: String, error_message: String) {
        if let Some(mut transaction) = self.bridge_transactions.get(&tx_hash) {
            transaction.status = TransactionStatus::Failed;
            transaction.updated_at = env::block_timestamp();
            self.bridge_transactions.insert(&tx_hash, &transaction);

            let failed_tx = FailedTransaction {
                tx_hash: tx_hash.clone(),
                error_message,
                failed_at: env::block_timestamp(),
                recovery_action: Some(RecoveryAction::Retry),
            };

            self.failed_transactions.insert(&tx_hash, &failed_tx);
            self.retry_queue.insert(&tx_hash);
        }
    }

    pub fn retry_transaction(&mut self, tx_hash: String) -> bool {
        if let Some(mut transaction) = self.bridge_transactions.get(&tx_hash) {
            if transaction.retry_count < self.alert_thresholds.max_retry_count {
                transaction.retry_count += 1;
                transaction.status = TransactionStatus::Initiated;
                transaction.updated_at = env::block_timestamp();
                self.bridge_transactions.insert(&tx_hash, &transaction);
                self.retry_queue.remove(&tx_hash);
                return true;
            }
        }
        false
    }

    pub fn get_failed_transactions(&self) -> Vec<FailedTransaction> {
        self.failed_transactions.values().collect()
    }

    pub fn get_transactions_by_user(&self, user: AccountId) -> Vec<BridgeTransaction> {
        self.bridge_transactions
            .values()
            .filter(|tx| tx.user == user)
            .collect()
    }

    pub fn get_stuck_transactions(&self) -> Vec<BridgeTransaction> {
        let current_time = env::block_timestamp();
        self.bridge_transactions
            .values()
            .filter(|tx| {
                matches!(
                    tx.status,
                    TransactionStatus::BridgeProcessing | TransactionStatus::TargetPending
                ) && (current_time - tx.updated_at) > self.alert_thresholds.stuck_transaction_threshold
            })
            .collect()
    }

    pub fn toggle_monitoring(&mut self, enabled: bool) {
        assert_eq!(env::predecessor_account_id(), self.owner_id, "Only owner can toggle monitoring");
        self.monitoring_enabled = enabled;
    }

    pub fn update_alert_thresholds(&mut self, thresholds: AlertThresholds) {
        assert_eq!(env::predecessor_account_id(), self.owner_id, "Only owner can update thresholds");
        self.alert_thresholds = thresholds;
    }
}