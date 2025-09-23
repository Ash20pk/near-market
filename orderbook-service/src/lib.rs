// Re-export main modules for the orderbook service library

use std::sync::Arc;
use tokio::sync::broadcast;

pub mod api;
pub mod matching;
pub mod storage;
pub mod near_client;
pub mod types;
pub mod solver_integration;
pub mod collateral;
pub mod ui;

pub use types::*;
use crate::matching::MatchingEngine;
use crate::storage::DatabaseTrait;
use crate::near_client::NearClient;
use crate::solver_integration::SolverIntegration;

#[derive(Clone)]
pub struct AppState {
    pub matching_engine: Arc<MatchingEngine>,
    pub database: Arc<dyn DatabaseTrait>,
    pub near_client: Arc<NearClient>,
    pub solver_integration: Arc<SolverIntegration>,
    pub ws_broadcaster: broadcast::Sender<WebSocketMessage>,
}