// Settlement manager for executing trades on NEAR using CTF contracts

use std::sync::Arc;
use std::collections::HashMap;
use tokio::sync::mpsc;
use tokio::time::{interval, Duration};
use uuid::Uuid;
use anyhow::Result;
use tracing::{info, warn, error};

use crate::types::{Trade, SettlementStatus, SettlementBatch, TradeType};
use crate::storage::DatabaseTrait;
use crate::near_client::NearClient;
use crate::collateral::CollateralManager;

pub struct SettlementManager {
    database: Arc<dyn DatabaseTrait>,
    near_client: Arc<NearClient>,
    collateral_manager: Arc<CollateralManager>,
    pending_settlements: HashMap<Uuid, SettlementBatch>,
}

impl SettlementManager {
    pub async fn new(
        database: Arc<dyn DatabaseTrait>,
        near_client: Arc<NearClient>,
    ) -> Result<Self> {
        let collateral_manager = Arc::new(
            CollateralManager::new(database.clone(), near_client.clone())
        );
        
        Ok(Self {
            database,
            near_client,
            collateral_manager,
            pending_settlements: HashMap::new(),
        })
    }

    pub async fn run(&self, mut trade_receiver: mpsc::UnboundedReceiver<Trade>) -> Result<()> {
        info!("Settlement manager started with ordered processing");

        // Create batch settlement timer (every 5 seconds)
        let mut settlement_timer = interval(Duration::from_secs(5));

        // Create retry timer (every 30 seconds)
        let mut retry_timer = interval(Duration::from_secs(30));

        // Use ordered queue for deterministic settlement
        let mut pending_trades = std::collections::VecDeque::new();
        let mut settlement_sequence = 0u64;

        loop {
            tokio::select! {
                // New trade to settle - maintain ordering
                trade = trade_receiver.recv() => {
                    if let Some(trade) = trade {
                        // Assign settlement sequence for ordering guarantees
                        settlement_sequence += 1;
                        info!("Queued trade {} for settlement (sequence: {})",
                              trade.trade_id, settlement_sequence);

                        pending_trades.push_back((trade, settlement_sequence));

                        // If we have enough trades, settle immediately with ordering
                        if pending_trades.len() >= 10 {
                            self.settle_trades_batch_ordered(
                                pending_trades.drain(..).collect()
                            ).await?;
                        }
                    }
                }

                // Periodic batch settlement with ordering
                _ = settlement_timer.tick() => {
                    if !pending_trades.is_empty() {
                        self.settle_trades_batch_ordered(
                            pending_trades.drain(..).collect()
                        ).await?;
                    }
                }

                // Retry failed settlements with ordering
                _ = retry_timer.tick() => {
                    self.retry_failed_settlements_ordered().await?;
                }
            }
        }
    }

    async fn settle_trades_batch_ordered(&self, trades_with_sequence: Vec<(Trade, u64)>) -> Result<()> {
        if trades_with_sequence.is_empty() {
            return Ok(());
        }

        info!("Settling ordered batch of {} trades", trades_with_sequence.len());

        // Sort by sequence to maintain strict ordering
        let mut sorted_trades = trades_with_sequence;
        sorted_trades.sort_by_key(|(_, sequence)| *sequence);

        // Group trades by type while preserving order within each type
        let mut direct_matches = Vec::new();
        let mut minting_trades = Vec::new();
        let mut burning_trades = Vec::new();

        for (trade, sequence) in sorted_trades {
            match trade.trade_type {
                TradeType::DirectMatch => direct_matches.push((trade, sequence)),
                TradeType::Minting => minting_trades.push((trade, sequence)),
                TradeType::Burning => burning_trades.push((trade, sequence)),
            }
        }

        // Settle each type in order - critical for Polymarket CLOB consistency
        if !minting_trades.is_empty() {
            self.settle_minting_trades_ordered(minting_trades).await?;
        }
        if !direct_matches.is_empty() {
            self.settle_direct_matches_ordered(direct_matches).await?;
        }
        if !burning_trades.is_empty() {
            self.settle_burning_trades_ordered(burning_trades).await?;
        }

        Ok(())
    }

    async fn settle_direct_matches_ordered(&self, trades_with_sequence: Vec<(Trade, u64)>) -> Result<()> {
        // Sort by sequence to maintain strict ordering
        let mut sorted_trades = trades_with_sequence;
        sorted_trades.sort_by_key(|(_, sequence)| *sequence);

        // Process direct trades in strict order to prevent race conditions
        for (trade, sequence) in sorted_trades {
            info!("⚡ Processing direct trade {} (sequence: {})", trade.trade_id, sequence);

            // Execute as atomic settlement transaction
            match self.execute_direct_settlement_transaction(trade).await {
                Ok(()) => {
                    info!("✅ Direct trade settlement completed (sequence: {})", sequence);
                }
                Err(e) => {
                    error!("❌ Direct trade settlement failed (sequence: {}): {}", sequence, e);
                }
            }
        }

        Ok(())
    }

    /// Execute direct settlement as atomic transaction
    async fn execute_direct_settlement_transaction(&self, trade: Trade) -> Result<()> {
        // Update status to settling
        self.update_trade_status(&trade, SettlementStatus::Settling).await?;

        // Call solver contract to execute the trade
        let tx_hash = self.near_client.execute_direct_trade(&trade).await
            .map_err(|e| {
                error!("Failed to settle direct trade {}: {}", trade.trade_id, e);
                e
            })?;

        // Update with transaction hash
        self.update_trade_settlement(&trade, SettlementStatus::Settled, Some(tx_hash.clone())).await?;
        info!("🎯 Direct trade {} settled: {}", trade.trade_id, tx_hash);

        Ok(())
    }

    async fn settle_minting_trades_ordered(&self, trades_with_sequence: Vec<(Trade, u64)>) -> Result<()> {
        // Group by condition_id while preserving sequence order
        let mut by_condition: HashMap<String, Vec<(Trade, u64)>> = HashMap::new();

        for (trade, sequence) in trades_with_sequence {
            by_condition.entry(trade.condition_id.clone())
                .or_default()
                .push((trade, sequence));
        }

        for (condition_id, mut condition_trades) in by_condition {
            // Sort by sequence within each condition for atomic settlement
            condition_trades.sort_by_key(|(_, sequence)| *sequence);
            let trades: Vec<Trade> = condition_trades.into_iter().map(|(trade, _)| trade).collect();

            // Atomic settlement transaction for this condition
            match self.execute_minting_settlement_transaction(&condition_id, trades).await {
                Ok(()) => {
                    info!("✅ Atomic minting settlement completed for condition {}", condition_id);
                }
                Err(e) => {
                    error!("❌ Atomic minting settlement failed for condition {}: {}", condition_id, e);
                }
            }
        }

        Ok(())
    }

    /// Execute minting settlement as atomic transaction
    async fn execute_minting_settlement_transaction(
        &self,
        condition_id: &str,
        trades: Vec<Trade>,
    ) -> Result<()> {
        // Update all trades to settling status atomically
        for trade in &trades {
            self.update_trade_status(trade, SettlementStatus::Settling).await?;
        }

        // Use CollateralManager to calculate settlement
        let settlement = self.collateral_manager.calculate_settlement(trades.clone()).await
            .map_err(|e| {
                error!("Failed to calculate collateral settlement for condition {}: {}", condition_id, e);
                e
            })?;

        // Execute the collateral-based settlement atomically
        let tx_hash = self.collateral_manager.execute_settlement(&settlement).await
            .map_err(|e| {
                error!("Failed to execute collateral settlement for condition {}: {}", condition_id, e);
                e
            })?;

        // Update all trades to settled status atomically
        for trade in trades {
            self.update_trade_settlement(&trade, SettlementStatus::Settled, Some(tx_hash.clone())).await?;
        }

        info!("🎯 Atomic collateral-based settlement for condition {} completed: {}", condition_id, tx_hash);
        Ok(())
    }

    async fn settle_burning_trades_ordered(&self, trades_with_sequence: Vec<(Trade, u64)>) -> Result<()> {
        // Group by condition_id while preserving sequence order
        let mut by_condition: HashMap<String, Vec<(Trade, u64)>> = HashMap::new();

        for (trade, sequence) in trades_with_sequence {
            by_condition.entry(trade.condition_id.clone())
                .or_default()
                .push((trade, sequence));
        }

        for (condition_id, mut condition_trades) in by_condition {
            // Sort by sequence within each condition
            condition_trades.sort_by_key(|(_, sequence)| *sequence);
            let trades: Vec<Trade> = condition_trades.into_iter().map(|(trade, _)| trade).collect();

            // Atomic settlement transaction for burning
            match self.execute_burning_settlement_transaction(&condition_id, trades).await {
                Ok(()) => {
                    info!("✅ Atomic burning settlement completed for condition {}", condition_id);
                }
                Err(e) => {
                    error!("❌ Atomic burning settlement failed for condition {}: {}", condition_id, e);
                }
            }
        }

        Ok(())
    }

    /// Execute burning settlement as atomic transaction
    async fn execute_burning_settlement_transaction(
        &self,
        condition_id: &str,
        trades: Vec<Trade>,
    ) -> Result<()> {
        let total_amount: u128 = trades.iter().map(|t| t.size).sum();

        // Update all trades to settling status atomically
        for trade in &trades {
            self.update_trade_status(trade, SettlementStatus::Settling).await?;
        }

        // Call CTF to merge positions atomically
        let tx_hash = self.near_client.merge_positions(condition_id, total_amount).await
            .map_err(|e| {
                error!("Failed to settle burning batch for condition {}: {}", condition_id, e);
                e
            })?;

        // Update all trades to settled status atomically
        for trade in trades {
            self.update_trade_settlement(&trade, SettlementStatus::Settled, Some(tx_hash.clone())).await?;
        }

        info!("🎯 Burning batch for condition {} settled: {}", condition_id, tx_hash);
        Ok(())
    }

    async fn retry_failed_settlements_ordered(&self) -> Result<()> {
        let failed_trades = self.database.get_failed_trades().await?;

        if failed_trades.is_empty() {
            return Ok(());
        }

        warn!("🔄 Retrying {} failed settlements with ordering", failed_trades.len());

        // Group by settlement type and assign retry sequence
        let mut direct_matches = Vec::new();
        let mut minting_trades = Vec::new();
        let mut burning_trades = Vec::new();
        let mut retry_sequence = 0u64;

        for trade in failed_trades {
            // Reset status to pending for retry
            self.update_trade_status(&trade, SettlementStatus::Pending).await?;

            retry_sequence += 1;
            match trade.trade_type {
                TradeType::DirectMatch => direct_matches.push((trade, retry_sequence)),
                TradeType::Minting => minting_trades.push((trade, retry_sequence)),
                TradeType::Burning => burning_trades.push((trade, retry_sequence)),
            }
        }

        // Retry each type with ordering
        if !minting_trades.is_empty() {
            self.settle_minting_trades_ordered(minting_trades).await?;
        }
        if !direct_matches.is_empty() {
            self.settle_direct_matches_ordered(direct_matches).await?;
        }
        if !burning_trades.is_empty() {
            self.settle_burning_trades_ordered(burning_trades).await?;
        }

        Ok(())
    }

    async fn update_trade_status(&self, trade: &Trade, status: SettlementStatus) -> Result<()> {
        self.database.update_trade_settlement_status(trade.trade_id, status, None).await
    }

    async fn update_trade_settlement(
        &self,
        trade: &Trade,
        status: SettlementStatus,
        tx_hash: Option<String>,
    ) -> Result<()> {
        self.database.update_trade_settlement_status(trade.trade_id, status, tx_hash).await
    }

    // Getter method for accessing near_client from MatchingEngine
    pub fn get_near_client(&self) -> &Arc<NearClient> {
        &self.near_client
    }
}