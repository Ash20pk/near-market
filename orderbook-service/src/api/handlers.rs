// HTTP API handlers

use axum::{
    extract::{Path, State, WebSocketUpgrade, ws::WebSocket},
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use futures_util::{SinkExt, StreamExt};
use serde_json::json;
use uuid::Uuid;
use chrono::Utc;
use tracing::{info, error};
use anyhow::Result;

use crate::types::{
    Order, SubmitOrderRequest, SubmitOrderResponse, CancelOrderRequest, TradeMatch, OrderStatus
};
use crate::AppState;
use serde::Deserialize;

pub async fn health_check() -> impl IntoResponse {
    Json(json!({
        "status": "healthy",
        "service": "orderbook",
        "timestamp": Utc::now()
    }))
}

pub async fn submit_order(
    State(state): State<AppState>,
    Json(request): Json<SubmitOrderRequest>,
) -> impl IntoResponse {
    info!("Received order submission: {:?}", request);

    // Validate request
    if let Err(e) = validate_order_request(&request) {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": e,
                "order_id": null
            }))
        ).into_response();
    }

    // Get market info to validate and get condition_id
    let condition_id = match state.near_client.get_market_condition_id(&request.market_id).await {
        Ok(Some(id)) => id,
        Ok(None) => {
            return (
                StatusCode::NOT_FOUND,
                Json(json!({
                    "error": "Market not found",
                    "order_id": null
                }))
            ).into_response();
        }
        Err(e) => {
            error!("Failed to get market info: {}", e);
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": "Failed to validate market",
                    "order_id": null
                }))
            ).into_response();
        }
    };

    // Create order
    let order_id = Uuid::new_v4();
    let user_account = request.user_account.clone();
    let order = Order {
        order_id,
        market_id: request.market_id,
        condition_id,
        user_account: request.user_account,
        outcome: request.outcome,
        side: request.side,
        order_type: request.order_type,
        price: request.price.unwrap_or(0), // Market orders use 0, will be filled at market price
        original_size: request.size,
        remaining_size: request.size,
        filled_size: 0,
        status: OrderStatus::Pending,
        created_at: Utc::now(),
        expires_at: request.expires_at,
        solver_account: request.solver_account,
    };

    // Submit to matching engine
    match state.matching_engine.submit_order(order).await {
        Ok(trades) => {
            let matches: Vec<TradeMatch> = trades.iter().map(|trade| {
                let counterparty = if trade.maker_account == user_account {
                    &trade.taker_account
                } else {
                    &trade.maker_account
                };

                TradeMatch {
                    trade_id: trade.trade_id,
                    counterparty: counterparty.clone(),
                    price: trade.price,
                    size: trade.size,
                    settlement_pending: true,
                }
            }).collect();

            let response = SubmitOrderResponse {
                order_id,
                status: if matches.is_empty() { "pending".to_string() } else { "partially_filled".to_string() },
                message: format!("Order submitted successfully with {} matches", matches.len()),
                matches,
            };

            info!("Order {} submitted successfully with {} matches", order_id, trades.len());

            (StatusCode::OK, Json(response)).into_response()
        }
        Err(e) => {
            error!("Failed to submit order: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": format!("Failed to submit order: {}", e),
                    "order_id": order_id
                }))
            ).into_response()
        }
    }
}

pub async fn cancel_order(
    State(state): State<AppState>,
    Path(order_id): Path<Uuid>,
    Json(request): Json<CancelOrderRequest>,
) -> impl IntoResponse {
    info!("Cancelling order: {}", order_id);

    // Verify order_id matches
    if request.order_id != order_id {
        return (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "error": "Order ID mismatch",
                "cancelled": false
            }))
        ).into_response();
    }

    match state.matching_engine.cancel_order(order_id, &request.user_account).await {
        Ok(cancelled) => {
            if cancelled {
                info!("Order {} cancelled successfully", order_id);
                (StatusCode::OK, Json(json!({
                    "message": "Order cancelled successfully",
                    "cancelled": true
                }))).into_response()
            } else {
                (StatusCode::BAD_REQUEST, Json(json!({
                    "error": "Order could not be cancelled",
                    "cancelled": false
                }))).into_response()
            }
        }
        Err(e) => {
            error!("Failed to cancel order {}: {}", order_id, e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": format!("Failed to cancel order: {}", e),
                    "cancelled": false
                }))
            ).into_response()
        }
    }
}

pub async fn get_orderbook(
    State(state): State<AppState>,
    Path((market_id, outcome)): Path<(String, u8)>,
) -> impl IntoResponse {
    match state.matching_engine.get_orderbook_snapshot(&market_id, outcome).await {
        Ok(Some(snapshot)) => {
            (StatusCode::OK, Json(snapshot)).into_response()
        }
        Ok(None) => {
            (StatusCode::NOT_FOUND, Json(json!({
                "error": "Orderbook not found"
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to get orderbook snapshot: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": format!("Failed to get orderbook: {}", e)
                }))
            ).into_response()
        }
    }
}

pub async fn get_market_price(
    State(state): State<AppState>,
    Path((market_id, outcome)): Path<(String, u8)>,
) -> impl IntoResponse {
    match state.matching_engine.get_market_price(&market_id, outcome).await {
        Ok(Some(price)) => {
            (StatusCode::OK, Json(price)).into_response()
        }
        Ok(None) => {
            (StatusCode::NOT_FOUND, Json(json!({
                "error": "Market not found"
            }))).into_response()
        }
        Err(e) => {
            error!("Failed to get market price: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({
                    "error": format!("Failed to get market price: {}", e)
                }))
            ).into_response()
        }
    }
}

pub async fn websocket_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
) -> Response {
    ws.on_upgrade(|socket| websocket_connection(socket, state))
}

async fn websocket_connection(socket: WebSocket, state: AppState) {
    info!("WebSocket connection established");

    let (mut ws_sender, mut ws_receiver) = socket.split();
    let mut broadcast_receiver = state.ws_broadcaster.subscribe();

    // Handle incoming WebSocket messages from client (if any)
    let client_task = tokio::spawn(async move {
        while let Some(msg) = ws_receiver.next().await {
            match msg {
                Ok(axum::extract::ws::Message::Text(text)) => {
                    info!("Received WebSocket message from client: {}", text);
                    // Could handle client commands here (subscribe to specific markets, etc.)
                }
                Ok(axum::extract::ws::Message::Close(_)) => {
                    info!("WebSocket connection closed by client");
                    break;
                }
                Err(e) => {
                    error!("WebSocket error: {}", e);
                    break;
                }
                _ => {}
            }
        }
    });

    // Handle broadcasting messages to client
    let broadcast_task = tokio::spawn(async move {
        while let Ok(message) = broadcast_receiver.recv().await {
            let json_message = match serde_json::to_string(&message) {
                Ok(json) => json,
                Err(e) => {
                    error!("Failed to serialize WebSocket message: {}", e);
                    continue;
                }
            };

            if let Err(e) = ws_sender.send(axum::extract::ws::Message::Text(json_message)).await {
                error!("Failed to send WebSocket message: {}", e);
                break;
            }
        }
    });

    // Wait for either task to complete (connection closed or error)
    tokio::select! {
        _ = client_task => {
            info!("WebSocket client task completed");
        }
        _ = broadcast_task => {
            info!("WebSocket broadcast task completed");
        }
    }

    info!("WebSocket connection closed");
}

fn validate_order_request(request: &SubmitOrderRequest) -> Result<(), String> {
    if request.market_id.is_empty() {
        return Err("Market ID cannot be empty".to_string());
    }
    
    if request.user_account.is_empty() {
        return Err("User account cannot be empty".to_string());
    }
    
    if request.solver_account.is_empty() {
        return Err("Solver account cannot be empty".to_string());
    }
    
    if request.outcome > 1 {
        return Err("Outcome must be 0 (NO) or 1 (YES)".to_string());
    }
    
    if request.size == 0 {
        return Err("Order size must be greater than 0".to_string());
    }
    
    // Validate price based on order type (cents format: 0-100)
    match request.order_type {
        crate::types::OrderType::Limit |
        crate::types::OrderType::GTC |
        crate::types::OrderType::GTD => {
            // Limit/GTC/GTD orders MUST have a valid price
            match request.price {
                Some(price) => {
                    // Use Polymarket-style tick size validation
                    let tick_config = crate::types::TickSizeConfig::default();
                    match tick_config.round_price(price) {
                        Ok(rounded_price) => {
                            if rounded_price != price {
                                return Err(format!("Price {} invalid for tick size. Use {}", price, rounded_price));
                            }
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                }
                None => {
                    return Err("Limit/GTC/GTD orders must specify a price between 100-99999 (0.001-0.99999)".to_string());
                }
            }
        }
        crate::types::OrderType::FOK |
        crate::types::OrderType::FAK => {
            // FOK/FAK orders MUST have a valid price
            match request.price {
                Some(price) => {
                    // Use Polymarket-style tick size validation
                    let tick_config = crate::types::TickSizeConfig::default();
                    match tick_config.round_price(price) {
                        Ok(rounded_price) => {
                            if rounded_price != price {
                                return Err(format!("Price {} invalid for tick size. Use {}", price, rounded_price));
                            }
                        }
                        Err(e) => {
                            return Err(e);
                        }
                    }
                }
                None => {
                    return Err("FOK/FAK orders must specify a price between 100-99999 (0.001-0.99999)".to_string());
                }
            }
        }
        crate::types::OrderType::Market => {
            // Market orders should NOT specify a price
            if request.price.is_some() && request.price != Some(0) {
                return Err("Market orders should not specify a price".to_string());
            }
        }
    }
    
    Ok(())
}

// ================================
// POLYMARKET-STYLE COLLATERAL API
// ================================

#[derive(Deserialize)]
pub struct CollateralBalanceRequest {
    pub account_id: String,
    pub market_id: String,
}

pub async fn get_collateral_balance(
    State(state): State<AppState>,
    Json(request): Json<CollateralBalanceRequest>,
) -> impl IntoResponse {
    info!("Getting collateral balance for {} in market {}", request.account_id, request.market_id);

    // Get balance from database through collateral manager
    match state.database.get_collateral_balance(&request.account_id, &request.market_id).await {
        Ok(Some(balance)) => {
            Json(json!({
                "balance": balance,
                "status": "success"
            }))
        }
        Ok(None) => {
            // Create demo balance for new users
            let balance = crate::types::CollateralBalance {
                account_id: request.account_id.clone(),
                market_id: request.market_id.clone(),
                available_balance: 1_000_000_000, // $1,000 USDC for demo
                reserved_balance: 0,
                position_balance: 0,
                total_deposited: 1_000_000_000,
                total_withdrawn: 0,
                last_updated: Utc::now(),
            };
            
            // Store the demo balance
            if let Err(e) = state.database.update_collateral_balance(&balance).await {
                error!("Failed to store demo balance: {}", e);
            }
            
            Json(json!({
                "balance": balance,
                "status": "success",
                "message": "Created demo balance with $1,000 USDC"
            }))
        }
        Err(e) => {
            error!("Database error getting collateral balance: {}", e);
            Json(json!({
                "error": "Database error",
                "status": "error"
            }))
        }
    }
}

#[derive(Deserialize)]
pub struct DepositCollateralRequest {
    pub account_id: String,
    pub market_id: String,
    pub amount: u128, // USDC amount in microunits
}

pub async fn deposit_collateral(
    State(_state): State<AppState>,
    Json(request): Json<DepositCollateralRequest>,
) -> impl IntoResponse {
    info!("Depositing {} USDC collateral for {} in market {}", 
        request.amount as f64 / 1_000_000.0,
        request.account_id, 
        request.market_id
    );

    // In a real implementation, this would:
    // 1. Verify USDC transfer from user's wallet
    // 2. Update user's collateral balance in database
    // 3. Emit events for tracking
    
    // For now, just return success
    Json(json!({
        "status": "success",
        "message": format!("Deposited {} USDC as collateral", request.amount as f64 / 1_000_000.0),
        "new_balance": request.amount
    }))
}

#[derive(Deserialize)]
pub struct RegisterMarketRequest {
    pub market_id: String,
    pub condition_id: String,
}

pub async fn register_market_condition(
    State(state): State<AppState>,
    Json(request): Json<RegisterMarketRequest>,
) -> impl IntoResponse {
    info!("Registering market {} with condition {}", request.market_id, request.condition_id);

    match state.near_client.register_market_condition(&request.market_id, &request.condition_id).await {
        Ok(_) => {
            // Update the "latest market" tracking file for solver/TUI sync
            if let Err(e) = update_latest_market_file(&request.market_id) {
                error!("Failed to update latest market file: {}", e);
            }

            Json(json!({
                "status": "success",
                "message": format!("Registered market {} with condition {}", request.market_id, request.condition_id)
            }))
        }
        Err(e) => {
            error!("Failed to register market: {}", e);
            Json(json!({
                "status": "error",
                "error": format!("Failed to register market: {}", e)
            }))
        }
    }
}

fn update_latest_market_file(market_id: &str) -> Result<()> {
    use std::fs;
    use chrono::Utc;

    let latest_market_info = serde_json::json!({
        "latest_market_id": market_id,
        "registered_at": Utc::now().to_rfc3339(),
        "timestamp": Utc::now().timestamp()
    });

    // Write to current directory (orderbook-service/) and parent directory for solver daemon
    let paths = vec!["latest_market.json", "../latest_market.json"];

    for path in paths {
        if let Err(e) = fs::write(path, latest_market_info.to_string()) {
            error!("Failed to write latest market to {}: {}", path, e);
        } else {
            info!("Updated latest market tracking: {} -> {}", path, market_id);
        }
    }

    Ok(())
}