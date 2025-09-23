// NEAR client using stable lower-level crates to avoid version conflicts

use anyhow::{anyhow, Result};
use serde_json::json;
use tracing::{info, error};
use std::sync::atomic::{AtomicU64, Ordering};
use std::collections::HashMap;
use std::sync::RwLock;
use std::str::FromStr;

use near_account_id::AccountId;
use near_crypto::{InMemorySigner, SecretKey, Signer};
use near_jsonrpc_client::{JsonRpcClient, methods};
use near_primitives::{
    transaction::{Action, FunctionCallAction, Transaction, SignedTransaction},
    types::{BlockReference, Finality},
    views::QueryRequest as ViewRequest,
    hash::hash,
};

use crate::types::{Trade, OrderSide};

pub struct NearClient {
    rpc_client: JsonRpcClient,
    signer_account: AccountId,
    signer: Signer,
    // Mock data for testing
    mock_markets: RwLock<HashMap<String, String>>, // market_id -> condition_id
    call_count: AtomicU64,
    total_gas_used: AtomicU64,
    failure_rate: RwLock<f64>,
    // Serialize TX creation/sending to avoid nonce races
    tx_lock: tokio::sync::Mutex<()>,
    nonce_tracker: tokio::sync::Mutex<Option<u64>>,
}

impl NearClient {
    pub async fn new() -> Result<Self> {
        // Load environment variables
        let signer_account_str = std::env::var("SIGNER_ACCOUNT_ID")
            .unwrap_or_else(|_| "ashpk20.testnet".to_string());
        let signer_account = AccountId::from_str(&signer_account_str)?;

        let private_key_str = std::env::var("PRIVATE_KEY")
            .map_err(|_| anyhow::anyhow!("PRIVATE_KEY environment variable required"))?;
        
        let private_key = SecretKey::from_str(&private_key_str)?;
        let signer = InMemorySigner::from_secret_key(signer_account.clone(), private_key).into();

        // Setup RPC client
        let rpc_url = std::env::var("NEAR_RPC_URL")
            .unwrap_or_else(|_| "https://rpc.testnet.near.org".to_string());
        let rpc_client = JsonRpcClient::connect(&rpc_url);

        // Initialize mock markets and load persisted condition IDs
        let mut mock_markets = HashMap::new();
        
        // Load persisted condition IDs from JSON file (test script writes these)
        if let Ok(contents) = std::fs::read_to_string("market_conditions.json") {
            if let Ok(persisted_markets) = serde_json::from_str::<HashMap<String, String>>(&contents) {
                for (market_id, condition_id) in persisted_markets {
                    mock_markets.insert(market_id.clone(), condition_id.clone());
                    info!("Loaded persisted condition mapping: {} → {}", market_id, condition_id);
                }
            }
        }
        
        // Add default mock markets for backwards compatibility
        for i in 1..=100 {
            let market_key = format!("market_{}", i);
            if !mock_markets.contains_key(&market_key) {
                mock_markets.insert(market_key, format!("condition_{}", i));
            }
        }

        info!("NEAR client initialized using NEAR JSON-RPC");
        info!("Signer account: {}", signer_account);
        info!("RPC URL: {}", rpc_url);

        Ok(Self {
            rpc_client,
            signer_account,
            signer,
            mock_markets: RwLock::new(mock_markets),
            call_count: AtomicU64::new(0),
            total_gas_used: AtomicU64::new(0),
            failure_rate: RwLock::new(0.0),
            tx_lock: tokio::sync::Mutex::new(()),
            nonce_tracker: tokio::sync::Mutex::new(None),
        })
    }

    pub async fn get_market_condition_id(&self, market_id: &str) -> Result<Option<String>> {
        // ONLY use registered/stored condition IDs - no generation
        let markets = self.mock_markets.read()
            .map_err(|e| anyhow!("Failed to acquire read lock on markets: {}", e))?;
        let result = markets.get(market_id).cloned();
        
        if let Some(condition_id) = &result {
            info!("Found registered condition ID for market {}: {}", market_id, condition_id);
        } else {
            info!("No condition ID registered for market: {}", market_id);
        }
        
        Ok(result)
    }

    pub async fn register_market_condition(&self, market_id: &str, condition_id: &str) -> Result<()> {
        {
            let mut markets = self.mock_markets.write()
                .map_err(|e| anyhow!("Failed to acquire write lock on markets: {}", e))?;
            markets.insert(market_id.to_string(), condition_id.to_string());
            
            // Persist to JSON file so mapping survives service restarts
            if let Ok(json_data) = serde_json::to_string_pretty(&*markets) {
                if let Err(e) = std::fs::write("market_conditions.json", json_data) {
                    error!("Failed to persist market conditions to JSON: {}", e);
                }
            }
        }
        
        info!("Registered and persisted market {} with condition {}", market_id, condition_id);
        Ok(())
    }

    pub async fn execute_direct_trade(&self, trade: &Trade) -> Result<String> {
        info!("Executing direct trade: {} @ {} between {} and {}", 
            trade.size, trade.price, trade.maker_account, trade.taker_account);

        match (&trade.maker_side, &trade.taker_side) {
            (OrderSide::Buy, OrderSide::Sell) | (OrderSide::Sell, OrderSide::Buy) => {
                self.execute_token_transfer(trade).await
            },
            _ => {
                error!("Invalid trade sides: {:?} vs {:?}", trade.maker_side, trade.taker_side);
                Err(anyhow::anyhow!("Invalid trade configuration"))
            }
        }
    }

    async fn execute_token_transfer(&self, trade: &Trade) -> Result<String> {
        let (seller, buyer) = if trade.maker_side == OrderSide::Sell {
            (&trade.maker_account, &trade.taker_account)
        } else {
            (&trade.taker_account, &trade.maker_account)
        };
        
        info!("Transferring {} CTF tokens from {} to {} at price {}", 
            trade.size, seller, buyer, trade.price);

        // Use proper deterministic position ID calculation
        let position_id = self.get_position_id_for_outcome(&trade.condition_id, trade.outcome).await?;
        let ctf_contract_str = std::env::var("CTF_CONTRACT_ID")
            .unwrap_or_else(|_| "ctf.ashpk20.testnet".to_string());
        let ctf_contract = AccountId::from_str(&ctf_contract_str)?;

        let args = json!({
            "from": seller,
            "to": buyer,
            "position_id": position_id,
            "amount": trade.size.to_string(),
            "data": "" // Match testPositionID reference
        });

        self.call_contract_function_commit(
            &ctf_contract,
            "safe_transfer_from",
            &args,
            150_000_000_000_000, // 150 TGas to match testPositionID reference
            0,
        ).await
    }

    pub async fn split_position(&self, condition_id: &str, amount: u128) -> Result<String> {
        info!("🔍 DEBUGGING: About to call split_position with:");
        info!("   condition_id: '{}'", condition_id);
        info!("   condition_id length: {}", condition_id.len());
        info!("   amount: {}", amount);

        let ctf_contract_str = std::env::var("CTF_CONTRACT_ID")
            .unwrap_or_else(|_| "ctf.ashpk20.testnet".to_string());
        let ctf_contract = AccountId::from_str(&ctf_contract_str)?;

        // For binary prediction markets, partition represents the index sets for NO (1) and YES (2) outcomes
        let args = json!({
            "collateral_token": "3e2210e1184b45b64c8a434c0a7e7b23cc04ea7eb7a6c3c32520d03d4afcb8af",
            "parent_collection_id": "",
            "condition_id": condition_id,
            "partition": ["1", "2"], // NO=1 (binary 001), YES=2 (binary 010) for binary outcomes
            "amount": amount.to_string()
        });

        self.call_contract_function_commit(
            &ctf_contract,
            "split_position",
            &args,
            150_000_000_000_000, // 150 TGas
            0, // No NEAR deposit required for split_position
        ).await
    }

    pub async fn merge_positions(&self, condition_id: &str, amount: u128) -> Result<String> {
        info!("Merging positions for condition {} amount {}", condition_id, amount);

        let ctf_contract_str = std::env::var("CTF_CONTRACT_ID")
            .unwrap_or_else(|_| "ctf.ashpk20.testnet".to_string());
        let ctf_contract = AccountId::from_str(&ctf_contract_str)?;

        // For binary prediction markets, partition represents the index sets for NO (1) and YES (2) outcomes  
        let args = json!({
            "collateral_token": "3e2210e1184b45b64c8a434c0a7e7b23cc04ea7eb7a6c3c32520d03d4afcb8af",
            "parent_collection_id": "",
            "condition_id": condition_id,
            "partition": ["1", "2"], // NO=1 (binary 001), YES=2 (binary 010) for binary outcomes
            "amount": amount.to_string()
        });

        self.call_contract_function_commit(
            &ctf_contract,
            "merge_positions",
            &args,
            150_000_000_000_000, // 150 TGas
            0, // No NEAR deposit required for merge_positions
        ).await
    }

    /// Calculate the correct position ID for a given outcome in a condition
    pub async fn get_position_id_for_outcome(&self, condition_id: &str, outcome: u8) -> Result<String> {
        let ctf_contract_str = std::env::var("CTF_CONTRACT_ID")
            .unwrap_or_else(|_| "ctf.ashpk20.testnet".to_string());
        let ctf_contract = AccountId::from_str(&ctf_contract_str)?;

        // Calculate index set for the outcome (binary outcomes: 1 for YES, 2 for NO)
        // outcome 0 (NO in orderbook) → index_set ["2"] → NO tokens
        // outcome 1 (YES in orderbook) → index_set ["1"] → YES tokens
        let index_set = vec![if outcome == 0 { "2" } else { "1" }];

        // Get collection ID for this outcome
        let collection_id: String = self.call_view_function(
            &ctf_contract,
            "get_collection_id",
            &json!({
                "parent_collection_id": "",
                "condition_id": condition_id,
                "index_set": index_set
            })
        ).await?;

        // Get position ID for this collection
        let position_id: String = self.call_view_function(
            &ctf_contract,
            "get_position_id",
            &json!({
                "collateral_token": "3e2210e1184b45b64c8a434c0a7e7b23cc04ea7eb7a6c3c32520d03d4afcb8af",
                "collection_id": collection_id
            })
        ).await?;

        Ok(position_id)
    }

    pub async fn transfer_position(&self, to_account: &str, position_id: &str, amount: u128) -> Result<String> {
        info!("Transferring {} units of position {} to {}", amount, position_id, to_account);

        let ctf_contract_str = std::env::var("CTF_CONTRACT_ID")
            .unwrap_or_else(|_| "ctf.ashpk20.testnet".to_string());
        let ctf_contract = AccountId::from_str(&ctf_contract_str)?;

        let args = json!({
            "from": self.signer_account.to_string(),
            "to": to_account,
            "position_id": position_id,
            "amount": amount.to_string(),
            "data": ""
        });

        self.call_contract_function_commit(
            &ctf_contract,
            "safe_transfer_from",
            &args,
            150_000_000_000_000, // 150 TGas
            0, // No NEAR deposit required for transfers
        ).await
    }

    pub async fn transfer_usdc(&self, from_account: &str, to_account: &str, amount: u128) -> Result<String> {
        info!("Transferring {} USDC from {} to {}", amount as f64 / 1_000_000.0, from_account, to_account);

        let usdc_contract_str = std::env::var("USDC_CONTRACT_ID")
            .unwrap_or_else(|_| "3e2210e1184b45b64c8a434c0a7e7b23cc04ea7eb7a6c3c32520d03d4afcb8af".to_string());
        let usdc_contract = AccountId::from_str(&usdc_contract_str)?;

        let args = json!({
            "receiver_id": to_account,
            "amount": amount.to_string(),
            "memo": "CLOB trade settlement"
        });

        self.call_contract_function(
            &usdc_contract,
            "ft_transfer",
            &args,
            50_000_000_000_000, // 50 TGas
            1, // 1 yoctoNEAR deposit required for ft_transfer
        ).await
    }

    async fn call_view_function<T: serde::de::DeserializeOwned>(
        &self,
        contract_id: &AccountId,
        method_name: &str,
        args: &serde_json::Value,
    ) -> Result<T> {
        use near_jsonrpc_client::methods;

        let request = methods::query::RpcQueryRequest {
            block_reference: near_primitives::types::BlockReference::latest(),
            request: near_primitives::views::QueryRequest::CallFunction {
                account_id: contract_id.clone(),
                method_name: method_name.to_string(),
                args: args.to_string().into_bytes().into(),
            },
        };

        let response = self.rpc_client.call(request).await
            .map_err(|e| anyhow!("NEAR view function call failed: {}", e))?;

        if let near_jsonrpc_primitives::types::query::QueryResponseKind::CallResult(result) = response.kind {
            let response: T = serde_json::from_slice(&result.result)
                .map_err(|e| anyhow!("Failed to deserialize view function response: {}", e))?;
            Ok(response)
        } else {
            Err(anyhow!("Unexpected query response type"))
        }
    }

    async fn call_contract_function(
        &self,
        contract_id: &AccountId,
        method_name: &str,
        args: &serde_json::Value,
        gas: u64,
        deposit: u128,
    ) -> Result<String> {
        info!("Calling NEAR contract: {}.{} with args: {}", contract_id, method_name, args);

        // Serialize TX creation/sending to avoid nonce races
        let _guard = self.tx_lock.lock().await;

        // Get current nonce 
        let access_key_query = methods::query::RpcQueryRequest {
            block_reference: BlockReference::Finality(Finality::Final),
            request: ViewRequest::ViewAccessKey {
                account_id: self.signer_account.clone(),
                public_key: self.signer.public_key(),
            },
        };

        let access_key_query_response = self.rpc_client
            .call(access_key_query)
            .await?;

        let current_nonce = if let near_jsonrpc_primitives::types::query::QueryResponseKind::AccessKey(access_key) = access_key_query_response.kind {
            access_key.nonce
        } else {
            return Err(anyhow::anyhow!("Failed to query access key"));
        };

        // Get latest block hash
        let block_request = methods::block::RpcBlockRequest {
            block_reference: BlockReference::Finality(Finality::Final),
        };

        let block_response = self.rpc_client
            .call(block_request)
            .await?;

        let block_hash = block_response.header.hash;

        // Create transaction
        let action = Action::FunctionCall(Box::new(FunctionCallAction {
            method_name: method_name.to_string(),
            args: args.to_string().into_bytes(),
            gas,
            deposit,
        }));

        let unsigned_transaction = Transaction::V0(near_primitives::transaction::TransactionV0 {
            signer_id: self.signer_account.clone(),
            public_key: self.signer.public_key(),
            nonce: current_nonce + 1,
            receiver_id: contract_id.clone(),
            block_hash,
            actions: vec![action],
        });

        // Sign transaction
        let transaction_hash = hash(&borsh::to_vec(&unsigned_transaction)
            .map_err(|e| anyhow!("Failed to serialize transaction: {}", e))?);
        let signature = self.signer.sign(&transaction_hash.as_ref());
        let signed_transaction = SignedTransaction::new(signature, unsigned_transaction);

        // Send transaction
        let tx_request = methods::broadcast_tx_async::RpcBroadcastTxAsyncRequest {
            signed_transaction,
        };

        let response = self.rpc_client
            .call(tx_request)
            .await?;

        let tx_hash = response.to_string();
        info!("Transaction sent: {}", tx_hash);

        // Increment counters
        self.call_count.fetch_add(1, Ordering::Relaxed);
        self.total_gas_used.fetch_add(gas, Ordering::Relaxed);

        Ok(tx_hash)
    }

    // New: commit-style sender that waits for finalization before returning
    pub async fn call_contract_function_commit(
        &self,
        contract_id: &AccountId,
        method_name: &str,
        args: &serde_json::Value,
        gas: u64,
        deposit: u128,
    ) -> Result<String> {
        info!("Calling NEAR contract (commit): {}.{} with args: {}", contract_id, method_name, args);

        // Serialize TX creation/sending to avoid nonce races - hold lock until completion
        let _guard = self.tx_lock.lock().await;

        let mut attempts = 0usize;
        loop {
            attempts += 1;

            // Get or calculate next nonce atomically
            let next_nonce = {
                let mut nonce_tracker = self.nonce_tracker.lock().await;

                match *nonce_tracker {
                    Some(last_used_nonce) => {
                        // Use incremented last known nonce
                        let next = last_used_nonce + 1;
                        *nonce_tracker = Some(next);
                        next
                    }
                    None => {
                        // First transaction - query the current nonce from network
                        let access_key_query = methods::query::RpcQueryRequest {
                            block_reference: BlockReference::Finality(Finality::Final),
                            request: ViewRequest::ViewAccessKey {
                                account_id: self.signer_account.clone(),
                                public_key: self.signer.public_key(),
                            },
                        };

                        let access_key_query_response = self.rpc_client
                            .call(access_key_query)
                            .await?;

                        let current_nonce = if let near_jsonrpc_primitives::types::query::QueryResponseKind::AccessKey(access_key) = access_key_query_response.kind {
                            access_key.nonce
                        } else {
                            return Err(anyhow::anyhow!("Failed to query access key"));
                        };

                        let next = current_nonce + 1;
                        *nonce_tracker = Some(next);
                        next
                    }
                }
            };

            // Get latest block hash
            let block_request = methods::block::RpcBlockRequest {
                block_reference: BlockReference::Finality(Finality::Final),
            };

            let block_response = self.rpc_client
                .call(block_request)
                .await?;

            let block_hash = block_response.header.hash;

            // Create transaction
            let action = Action::FunctionCall(Box::new(FunctionCallAction {
                method_name: method_name.to_string(),
                args: args.to_string().into_bytes(),
                gas,
                deposit,
            }));

            let unsigned_transaction = Transaction::V0(near_primitives::transaction::TransactionV0 {
                signer_id: self.signer_account.clone(),
                public_key: self.signer.public_key(),
                nonce: next_nonce,
                receiver_id: contract_id.clone(),
                block_hash,
                actions: vec![action],
            });

            // Sign transaction
            let transaction_hash = hash(&borsh::to_vec(&unsigned_transaction)
                .map_err(|e| anyhow!("Failed to serialize transaction: {}", e))?);
            let signature = self.signer.sign(&transaction_hash.as_ref());
            let signed_transaction = SignedTransaction::new(signature, unsigned_transaction);

            // Send transaction and wait for finalization
            let tx_request = methods::broadcast_tx_commit::RpcBroadcastTxCommitRequest {
                signed_transaction,
            };

            let result = self.rpc_client.call(tx_request).await;

            match result {
                Ok(outcome) => {
                    let tx_hash_str = outcome.transaction_outcome.id.to_string();
                    info!("Transaction committed with nonce {}: {}", next_nonce, tx_hash_str);

                    self.call_count.fetch_add(1, Ordering::Relaxed);
                    self.total_gas_used.fetch_add(gas, Ordering::Relaxed);
                    return Ok(tx_hash_str);
                }
                Err(e) => {
                    let msg = format!("{}", e);
                    if msg.contains("InvalidNonce") && attempts < 3 {
                        info!("InvalidNonce detected for nonce {}, refreshing from network (attempt {} of 3)", next_nonce, attempts + 1);

                        // Reset nonce tracker to force fresh query from network
                        {
                            let mut nonce_tracker = self.nonce_tracker.lock().await;
                            *nonce_tracker = None;
                        }

                        // Longer backoff to allow network state to settle
                        tokio::time::sleep(std::time::Duration::from_millis(200 * attempts as u64)).await;
                        continue;
                    }
                    return Err(anyhow!("NEAR commit call failed: {}", msg));
                }
            }
        }
    }

    // Legacy method for backward compatibility
    pub async fn call_near_contract(
        &self,
        contract_id: &str,
        method_name: &str,
        args: &str,
        gas: &str,
        deposit: &str,
    ) -> Result<String> {
        let contract_account = AccountId::from_str(contract_id)?;
        let gas_amount: u64 = gas.parse().unwrap_or(30_000_000_000_000);
        let deposit_amount: u128 = deposit.parse().unwrap_or(0);
        let args_json: serde_json::Value = serde_json::from_str(args)?;
        
        self.call_contract_function(
            &contract_account,
            method_name,
            &args_json,
            gas_amount,
            deposit_amount,
        ).await
    }

    pub async fn get_call_count(&self) -> u64 {
        self.call_count.load(Ordering::Relaxed)
    }

    pub async fn get_total_gas_used(&self) -> u64 {
        self.total_gas_used.load(Ordering::Relaxed)
    }

    pub async fn set_failure_rate(&self, rate: f64) {
        let mut failure_rate = self.failure_rate.write().unwrap();
        *failure_rate = rate;
    }

    // Balance checking methods for trade type determination
    pub async fn get_usdc_balance(&self, account_id: &str) -> Result<u128> {
        let usdc_contract_str = std::env::var("USDC_CONTRACT_ID")
            .unwrap_or_else(|_| "3e2210e1184b45b64c8a434c0a7e7b23cc04ea7eb7a6c3c32520d03d4afcb8af".to_string());
        let usdc_contract = AccountId::from_str(&usdc_contract_str)?;

        let args = json!({
            "account_id": account_id
        });

        let request = methods::query::RpcQueryRequest {
            block_reference: BlockReference::Finality(Finality::Final),
            request: ViewRequest::CallFunction {
                account_id: usdc_contract.clone(),
                method_name: "ft_balance_of".to_string(),
                args: args.to_string().into_bytes().into(),
            },
        };

        let response = self.rpc_client.call(request).await?;

        if let near_jsonrpc_primitives::types::query::QueryResponseKind::CallResult(result) = response.kind {
            let balance_str = String::from_utf8(result.result)?;
            // Remove quotes if present (NEAR returns "123456" not 123456)
            let balance_clean = balance_str.trim_matches('"');
            let balance: u128 = balance_clean.parse()
                .unwrap_or(0);
            
            Ok(balance)
        } else {
            Ok(0)
        }
    }

    pub async fn get_ctf_token_balance(&self, account_id: &str, position_id: &str) -> Result<u128> {
        let ctf_contract_str = std::env::var("CTF_CONTRACT_ID")
            .unwrap_or_else(|_| "ctf.ashpk20.testnet".to_string());
        let ctf_contract = AccountId::from_str(&ctf_contract_str)?;

        let args = json!({
            "owner": account_id,
            "position_id": position_id
        });

        let request = methods::query::RpcQueryRequest {
            block_reference: BlockReference::Finality(Finality::Final),
            request: ViewRequest::CallFunction {
                account_id: ctf_contract.clone(),
                method_name: "balance_of".to_string(),
                args: args.to_string().into_bytes().into(),
            },
        };

        let response = self.rpc_client.call(request).await?;

        if let near_jsonrpc_primitives::types::query::QueryResponseKind::CallResult(result) = response.kind {
            let balance_str = String::from_utf8(result.result)?;
            // Remove quotes if present
            let balance_clean = balance_str.trim_matches('"');
            let balance: u128 = balance_clean.parse()
                .unwrap_or(0);
            
            Ok(balance)
        } else {
            Ok(0)
        }
    }

    pub async fn has_sufficient_outcome_tokens(&self, account_id: &str, market_id: &str, outcome: u8, required_amount: u128) -> Result<bool> {
        let position_id = format!("{}:{}", market_id, outcome);
        let balance = self.get_ctf_token_balance(account_id, &position_id).await?;
        Ok(balance >= required_amount)
    }

    pub async fn has_sufficient_usdc(&self, account_id: &str, required_amount: u128) -> Result<bool> {
        let balance = self.get_usdc_balance(account_id).await?;
        Ok(balance >= required_amount)
    }

}