use std::collections::{VecDeque, HashMap};
use std::time::{Duration, Instant};
use anyhow::Result;

use crossterm::event::{self, Event as CEvent, KeyCode};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::{Constraint, Direction, Layout, Alignment, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Table, Row, Cell};
use ratatui::{Terminal, Frame};
use tokio::sync::{mpsc, watch};
use tracing::{Event, Subscriber};
use tracing::field::{Field, Visit};
use std::fmt::Write as _;
use tracing_subscriber::layer::{Context, Layer};

use crate::types::PriceLevel;

#[derive(Clone, Debug, Default)]
pub struct MetricsSnapshot {
    pub orders_processed: u64,
    pub matches_executed: u64,
    pub best_bid: Option<f64>,
    pub best_ask: Option<f64>,
    pub p50_latency_ms: f64,
    pub p95_latency_ms: f64,
    pub p99_latency_ms: f64,
    pub orderbook_data: Option<OrderbookData>,
}

#[derive(Clone, Debug, Default)]
pub struct OrderbookData {
    pub market_id: String,
    pub outcome: u8,
    pub bids: Vec<PriceLevel>,
    pub asks: Vec<PriceLevel>,
    pub last_trade_price: Option<u64>,
    pub spread: Option<f64>,
    pub markets_with_activity: Vec<(String, usize, usize)>, // (market_id, bids, asks)
}

#[derive(Clone, Debug)]
pub struct SolverLogEntry {
    pub timestamp: Instant,
    pub level: String,
    pub message: String,
    pub source: String,
}

#[derive(Clone, Debug)]
pub struct AnimatedOrder {
    pub price_level: PriceLevel,
    pub side: OrderSide,
    pub animation_state: AnimationState,
    pub created_at: Instant,
    pub expires_at: Instant,
}

#[derive(Clone, Debug, PartialEq)]
pub enum OrderSide {
    Bid,
    Ask,
}

#[derive(Clone, Debug)]
pub enum AnimationState {
    Appearing { progress: f32 },     // 0.0 to 1.0
    Stable,                          // Fully visible
    Disappearing { progress: f32 },  // 1.0 to 0.0
}

#[derive(Default)]
pub struct OrderbookAnimator {
    pub animated_orders: HashMap<String, AnimatedOrder>,
    pub last_snapshot: Option<OrderbookData>,
    pub trade_flash: Option<TradeFlash>,
    pub ghost_orders: Vec<GhostOrder>, // Show recently executed orders
}

#[derive(Clone, Debug)]
pub struct GhostOrder {
    pub price: u64,
    pub size: u64,
    pub side: OrderSide,
    pub created_at: Instant,
    pub expires_at: Instant,
    pub was_consumed: bool, // True if order was completely filled
}

#[derive(Clone, Debug)]
pub struct TradeFlash {
    pub price: u64,
    pub size: u64,
    pub side: OrderSide,
    pub created_at: Instant,
    pub expires_at: Instant,
}

pub struct LogForwarderLayer {
    sender: mpsc::Sender<String>,
}

impl LogForwarderLayer {
    pub fn new(sender: mpsc::Sender<String>) -> Self {
        Self { sender }
    }
}

struct StringVisitor {
    message: String,
}

impl StringVisitor {
    fn new() -> Self {
        Self { message: String::new() }
    }
}

impl Visit for StringVisitor {
    fn record_str(&mut self, field: &Field, value: &str) {
        if field.name() == "message" {
            let _ = write!(&mut self.message, "{}", value);
        } else {
            let _ = write!(&mut self.message, " {}={} ", field.name(), value);
        }
    }

    fn record_debug(&mut self, field: &Field, value: &dyn std::fmt::Debug) {
        if field.name() == "message" {
            let _ = write!(&mut self.message, "{:?}", value);
        } else {
            let _ = write!(&mut self.message, " {}={:?} ", field.name(), value);
        }
    }
}

impl<S> Layer<S> for LogForwarderLayer
where
    S: Subscriber,
{
    fn on_event(&self, event: &Event<'_>, _ctx: Context<'_, S>) {
        let meta = event.metadata();
        let mut visitor = StringVisitor::new();
        event.record(&mut visitor);
        let mut msg = String::new();
        let _ = write!(msg, "{}: {}", meta.level(), visitor.message);
        let _ = self.sender.try_send(msg);
    }
}

pub async fn run_dashboard(
    mut log_rx: mpsc::Receiver<String>,
    metrics_rx: watch::Receiver<MetricsSnapshot>,
) -> Result<()> {
    // Terminal setup
    enable_raw_mode()?;
    std::io::stdout().execute(EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(std::io::stdout());
    let mut terminal = Terminal::new(backend)?;

    let mut solver_logs: VecDeque<SolverLogEntry> = VecDeque::with_capacity(500);
    let mut system_logs: VecDeque<String> = VecDeque::with_capacity(500);
    let mut orderbook_data = OrderbookData::default();
    let mut animator = OrderbookAnimator::default();
    let mut last_draw = Instant::now();
    let mut last_solver_log_check = Instant::now();

    loop {
        // Non-blocking drain of orderbook service logs
        while let Ok(log) = log_rx.try_recv() {
            // All logs from orderbook service go to system logs
            if system_logs.len() >= 500 {
                system_logs.pop_front();
            }
            system_logs.push_back(log);
        }

        // Read solver daemon logs from file every 2 seconds
        if last_solver_log_check.elapsed() >= Duration::from_millis(2000) {
            read_solver_logs(&mut solver_logs);
            last_solver_log_check = Instant::now();
        }

        // Draw at ~20 FPS max
        if last_draw.elapsed() >= Duration::from_millis(50) {
            let metrics = metrics_rx.borrow().clone();

            // Use real orderbook data and update animator
            if let Some(ref real_orderbook) = metrics.orderbook_data {
                // Update animator with new orderbook data
                animator.update_orderbook(&real_orderbook);
                orderbook_data = real_orderbook.clone();
            } else {
                // Clear orderbook data when no real data is available
                orderbook_data = OrderbookData::default();
            }

            // Update animation states
            animator.update_animations();

            terminal.draw(|f| {
                let main_layout = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Min(0), Constraint::Length(3)])
                    .split(f.size());

                let main_chunks = Layout::default()
                    .direction(Direction::Horizontal)
                    .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
                    .split(main_layout[0]);

                // Left side: Orderbook display
                render_orderbook_panel(f, main_chunks[0], &orderbook_data, &metrics, &animator);

                // Right side: Split between solver logs and system metrics
                let right_chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
                    .split(main_chunks[1]);

                // Top right: Solver logs
                render_solver_logs(f, right_chunks[0], &solver_logs);

                // Bottom right: System metrics and status
                render_system_metrics(f, right_chunks[1], &metrics, &system_logs);

                // Status bar with keyboard shortcuts
                render_status_bar(f, main_layout[1]);
            })?;
            last_draw = Instant::now();
        }

        // Handle input with a small timeout
        if crossterm::event::poll(Duration::from_millis(10))? {
            if let CEvent::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Esc => break,
                    KeyCode::Char('r') => {
                        // Refresh/clear logs
                        solver_logs.clear();
                        system_logs.clear();
                    }
                    KeyCode::Char('h') => {
                        // Show help - could be implemented as overlay
                        system_logs.push_back("Help: q=quit, r=refresh logs, h=help".to_string());
                    }
                    KeyCode::Char('c') => {
                        // Clear orderbook data to force refresh
                        orderbook_data = OrderbookData::default();
                    }
                    _ => {}
                }
            }
        }

        // Check if the sender side dropped; if both channels are closed, we can quit.
        if log_rx.is_closed() && metrics_rx.has_changed().is_err() {
            break;
        }
    }

    // Restore terminal
    disable_raw_mode()?;
    std::io::stdout().execute(LeaveAlternateScreen)?;
    Ok(())
}

fn render_orderbook_panel(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    orderbook: &OrderbookData,
    metrics: &MetricsSnapshot,
    animator: &OrderbookAnimator,
) {
    let orderbook_chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3),  // Header
            Constraint::Min(8),     // Orderbook table
            Constraint::Length(4),  // Market info
        ])
        .split(area);

    // Header with market info
    let header_text = vec![
        Line::from(vec![
            Span::styled("NEAR Prediction Market Orderbook",
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        ]),
        Line::from(format!("Active Market: {} | Outcome: {} | Orders: {}B/{}A",
            if orderbook.market_id.is_empty() {
                "Waiting for data...".to_string()
            } else if orderbook.market_id.len() > 30 {
                format!("...{}", &orderbook.market_id[orderbook.market_id.len()-25..])
            } else {
                orderbook.market_id.clone()
            },
            if orderbook.market_id.is_empty() { "N/A".to_string() } else { orderbook.outcome.to_string() },
            orderbook.bids.len(),
            orderbook.asks.len())),

        // Show additional markets with activity
        if orderbook.markets_with_activity.len() > 1 {
            Line::from(format!("Total Markets Active: {} | Others: {}",
                orderbook.markets_with_activity.len(),
                orderbook.markets_with_activity.iter()
                    .skip(1)
                    .take(2)
                    .map(|(id, b, a)| format!("{}({}B/{}A)",
                        if id.len() > 15 { &id[id.len()-12..] } else { id },
                        b, a))
                    .collect::<Vec<_>>()
                    .join(", ")
            ))
        } else {
            Line::from("Multi-Market Monitoring: Waiting for concurrent activity...")
        },
    ];
    let header = Paragraph::new(header_text)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);
    f.render_widget(header, orderbook_chunks[0]);

    // Orderbook table
    let orderbook_rows = create_animated_orderbook_rows(orderbook, animator);
    let orderbook_table = Table::new(
        orderbook_rows,
        [
            Constraint::Length(12), // Price
            Constraint::Length(15), // Size
            Constraint::Length(8),  // Count
            Constraint::Length(4),  // Side
            Constraint::Length(15), // Size
            Constraint::Length(12), // Price
        ]
    )
    .header(
        Row::new(vec!["Bid Price", "Bid Size", "Orders", "", "Ask Size", "Ask Price"])
            .style(Style::default().add_modifier(Modifier::BOLD))
            .bottom_margin(1),
    )
    .block(Block::default().title("Order Book").borders(Borders::ALL));
    f.render_widget(orderbook_table, orderbook_chunks[1]);

    // Market statistics
    let spread = orderbook.spread.map(|s| format!("{:.4}", s)).unwrap_or_else(|| "N/A".to_string());
    let last_price = orderbook.last_trade_price.map(|p| format!("{:.4}", p as f64 / 100000.0)).unwrap_or_else(|| "N/A".to_string());
    let best_bid = metrics.best_bid.map(|b| format!("{:.4}", b)).unwrap_or_else(|| "N/A".to_string());
    let best_ask = metrics.best_ask.map(|a| format!("{:.4}", a)).unwrap_or_else(|| "N/A".to_string());

    let market_info = vec![
        Line::from(format!("Last Price: ${} | Spread: ${}", last_price, spread)),
        Line::from(format!("Best Bid: ${} | Best Ask: ${}", best_bid, best_ask)),
    ];
    let market_stats = Paragraph::new(market_info)
        .block(Block::default().title("Market Stats").borders(Borders::ALL));
    f.render_widget(market_stats, orderbook_chunks[2]);
}

fn render_solver_logs(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    solver_logs: &VecDeque<SolverLogEntry>,
) {
    let log_items: Vec<ListItem> = solver_logs
        .iter()
        .rev()
        .take((area.height as usize).saturating_sub(2))
        .map(|entry| {
            let style = match entry.level.as_str() {
                "ERROR" => Style::default().fg(Color::Red),
                "WARN" => Style::default().fg(Color::Yellow),
                "INFO" => Style::default().fg(Color::Green),
                _ => Style::default().fg(Color::Gray),
            };

            let truncated_msg = if entry.message.len() > 60 {
                format!("{}...", &entry.message[..57])
            } else {
                entry.message.clone()
            };

            ListItem::new(Line::from(vec![
                Span::styled(format!("[{}] ", entry.level), style),
                Span::raw(truncated_msg),
            ]))
        })
        .collect();

    let solver_log_list = List::new(log_items)
        .block(Block::default()
            .title(format!("Solver Logs ({})", solver_logs.len()))
            .borders(Borders::ALL));
    f.render_widget(solver_log_list, area);
}

fn render_system_metrics(
    f: &mut Frame,
    area: ratatui::layout::Rect,
    metrics: &MetricsSnapshot,
    system_logs: &VecDeque<String>,
) {
    let metrics_chunks = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([Constraint::Percentage(60), Constraint::Percentage(40)])
        .split(area);

    // Performance metrics
    let perf_lines = vec![
        Line::from(vec![Span::styled("System Performance",
            Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))]),
        Line::from(format!("Orders: {}", metrics.orders_processed)),
        Line::from(format!("Matches: {}", metrics.matches_executed)),
        Line::from(format!("p50: {:.1}ms", metrics.p50_latency_ms)),
        Line::from(format!("p95: {:.1}ms", metrics.p95_latency_ms)),
    ];
    let perf_panel = Paragraph::new(perf_lines)
        .block(Block::default().title("Metrics").borders(Borders::ALL));
    f.render_widget(perf_panel, metrics_chunks[0]);

    // Recent system logs (last few) - show more relevant logs
    let recent_logs: Vec<ListItem> = system_logs
        .iter()
        .rev()
        .filter(|log| {
            // Filter for important system events
            log.contains("Order") || log.contains("Trade") || log.contains("submitted") ||
            log.contains("SOLVER") || log.contains("Multi-market") || log.contains("TUI")
        })
        .take(4)
        .map(|log| {
            let style = if log.contains("ERROR") {
                Style::default().fg(Color::Red)
            } else if log.contains("Trade") || log.contains("submitted") {
                Style::default().fg(Color::Green)
            } else if log.contains("SOLVER") {
                Style::default().fg(Color::Cyan)
            } else {
                Style::default().fg(Color::Gray)
            };

            let truncated = if log.len() > 35 {
                format!("{}...", &log[..32])
            } else {
                log.clone()
            };
            ListItem::new(Line::from(Span::styled(truncated, style)))
        })
        .collect();

    let system_log_list = List::new(recent_logs)
        .block(Block::default().title("System").borders(Borders::ALL));
    f.render_widget(system_log_list, metrics_chunks[1]);
}

fn create_animated_orderbook_rows(orderbook: &OrderbookData, animator: &OrderbookAnimator) -> Vec<Row<'static>> {
    let max_levels = 15;
    let mut rows = Vec::new();

    // Get current bids and asks
    let mut all_bids = orderbook.bids.clone();
    let mut all_asks = orderbook.asks.clone();

    // Add animated orders that might be disappearing
    for (_, animated) in &animator.animated_orders {
        match animated.side {
            OrderSide::Bid => {
                if matches!(animated.animation_state, AnimationState::Disappearing { .. }) {
                    all_bids.push(animated.price_level.clone());
                }
            }
            OrderSide::Ask => {
                if matches!(animated.animation_state, AnimationState::Disappearing { .. }) {
                    all_asks.push(animated.price_level.clone());
                }
            }
        }
    }

    // Add ghost orders (recently executed orders for visibility)
    for ghost in &animator.ghost_orders {
        let ghost_price_level = crate::types::PriceLevel {
            price: ghost.price,
            size: ghost.size as u128,
            order_count: 1,
        };

        match ghost.side {
            OrderSide::Bid => all_bids.push(ghost_price_level),
            OrderSide::Ask => all_asks.push(ghost_price_level),
        }
    }

    // Sort orders properly
    all_bids.sort_by(|a, b| b.price.cmp(&a.price)); // Highest bid first
    all_asks.sort_by(|a, b| a.price.cmp(&b.price)); // Lowest ask first

    let max_rows = std::cmp::max(all_bids.len(), all_asks.len()).min(max_levels);

    for i in 0..max_rows {
        let bid_style = if let Some(bid) = all_bids.get(i) {
            get_animated_style(&OrderSide::Bid, bid.price, animator)
        } else {
            Style::default()
        };

        let ask_style = if let Some(ask) = all_asks.get(i) {
            get_animated_style(&OrderSide::Ask, ask.price, animator)
        } else {
            Style::default()
        };

        let bid_price = all_bids.get(i).map(|b| format!("{:.4}", b.price as f64 / 100000.0)).unwrap_or_else(|| "".to_string());
        let bid_size = all_bids.get(i).map(|b| format!("{:.2}", b.size as f64 / 1_000_000.0)).unwrap_or_else(|| "".to_string());
        let bid_count = all_bids.get(i).map(|b| b.order_count.to_string()).unwrap_or_else(|| "".to_string());

        let ask_price = all_asks.get(i).map(|a| format!("{:.4}", a.price as f64 / 100000.0)).unwrap_or_else(|| "".to_string());
        let ask_size = all_asks.get(i).map(|a| format!("{:.2}", a.size as f64 / 1_000_000.0)).unwrap_or_else(|| "".to_string());

        // Add trade flash effect if there's a recent trade at this price level
        let trade_flash_style = animator.trade_flash.as_ref()
            .and_then(|flash| {
                if all_bids.get(i).map(|b| b.price).unwrap_or(0) == flash.price ||
                   all_asks.get(i).map(|a| a.price).unwrap_or(0) == flash.price {
                    Some(Style::default().bg(Color::Yellow).fg(Color::Black))
                } else {
                    None
                }
            });

        let final_bid_style = trade_flash_style.unwrap_or(bid_style.fg(Color::Green));
        let final_ask_style = trade_flash_style.unwrap_or(ask_style.fg(Color::Red));

        let row = Row::new(vec![
            Cell::from(bid_price).style(final_bid_style),
            Cell::from(bid_size).style(final_bid_style),
            Cell::from(bid_count).style(final_bid_style),
            Cell::from(" | ").style(Style::default().fg(Color::Gray)),
            Cell::from(ask_size).style(final_ask_style),
            Cell::from(ask_price).style(final_ask_style),
        ]);
        rows.push(row);
    }

    if rows.is_empty() {
        rows.push(Row::new(vec![
            Cell::from("").style(Style::default().fg(Color::Gray)),
            Cell::from("No bids").style(Style::default().fg(Color::Gray)),
            Cell::from("").style(Style::default().fg(Color::Gray)),
            Cell::from(" | ").style(Style::default().fg(Color::Gray)),
            Cell::from("No asks").style(Style::default().fg(Color::Gray)),
            Cell::from("").style(Style::default().fg(Color::Gray)),
        ]));
        rows.push(Row::new(vec![
            Cell::from("").style(Style::default().fg(Color::Gray)),
            Cell::from("").style(Style::default().fg(Color::Gray)),
            Cell::from("").style(Style::default().fg(Color::Gray)),
            Cell::from(" | ").style(Style::default().fg(Color::Gray)),
            Cell::from("").style(Style::default().fg(Color::Gray)),
            Cell::from("").style(Style::default().fg(Color::Gray)),
        ]));
        rows.push(Row::new(vec![
            Cell::from("").style(Style::default().fg(Color::Yellow)),
            Cell::from("Waiting for orders...").style(Style::default().fg(Color::Yellow)),
            Cell::from("").style(Style::default().fg(Color::Yellow)),
            Cell::from(" | ").style(Style::default().fg(Color::Gray)),
            Cell::from("Waiting for orders...").style(Style::default().fg(Color::Yellow)),
            Cell::from("").style(Style::default().fg(Color::Yellow)),
        ]));
    }

    rows
}

fn get_animated_style(side: &OrderSide, price: u64, animator: &OrderbookAnimator) -> Style {
    let price_key = format!("{:?}_{}", side, price);

    // Check if this is a ghost order (recently executed)
    if let Some(_ghost) = animator.ghost_orders.iter().find(|g| g.price == price && g.side == *side) {
        // Ghost orders have special styling - dimmed and italic to show they were recently executed
        match side {
            OrderSide::Bid => Style::default()
                .fg(Color::Rgb(100, 200, 100)) // Lighter green
                .add_modifier(Modifier::DIM)
                .add_modifier(Modifier::ITALIC),
            OrderSide::Ask => Style::default()
                .fg(Color::Rgb(200, 100, 100)) // Lighter red
                .add_modifier(Modifier::DIM)
                .add_modifier(Modifier::ITALIC),
        }
    } else if let Some(animated) = animator.animated_orders.get(&price_key) {
        match animated.animation_state {
            AnimationState::Appearing { progress } => {
                let alpha = (progress * 255.0) as u8;
                match side {
                    OrderSide::Bid => Style::default().fg(Color::Rgb(0, alpha, 0)),
                    OrderSide::Ask => Style::default().fg(Color::Rgb(alpha, 0, 0)),
                }
            }
            AnimationState::Stable => {
                match side {
                    OrderSide::Bid => Style::default().fg(Color::Green),
                    OrderSide::Ask => Style::default().fg(Color::Red),
                }
            }
            AnimationState::Disappearing { progress } => {
                let alpha = (progress * 255.0) as u8;
                match side {
                    OrderSide::Bid => Style::default().fg(Color::Rgb(0, alpha, 0)).add_modifier(Modifier::DIM),
                    OrderSide::Ask => Style::default().fg(Color::Rgb(alpha, 0, 0)).add_modifier(Modifier::DIM),
                }
            }
        }
    } else {
        match side {
            OrderSide::Bid => Style::default().fg(Color::Green),
            OrderSide::Ask => Style::default().fg(Color::Red),
        }
    }
}

fn create_orderbook_rows(orderbook: &OrderbookData) -> Vec<Row> {
    let max_levels = 15;
    let mut rows = Vec::new();

    // Get top bids and asks
    let bids = &orderbook.bids;
    let asks = &orderbook.asks;

    let max_rows = std::cmp::max(bids.len(), asks.len()).min(max_levels);

    for i in 0..max_rows {
        let bid_price = bids.get(i).map(|b| format!("{:.4}", b.price as f64 / 100000.0)).unwrap_or_else(|| "".to_string());
        let bid_size = bids.get(i).map(|b| format!("{:.2}", b.size as f64 / 1_000_000.0)).unwrap_or_else(|| "".to_string());
        let bid_count = bids.get(i).map(|b| b.order_count.to_string()).unwrap_or_else(|| "".to_string());

        let ask_price = asks.get(i).map(|a| format!("{:.4}", a.price as f64 / 100000.0)).unwrap_or_else(|| "".to_string());
        let ask_size = asks.get(i).map(|a| format!("{:.2}", a.size as f64 / 1_000_000.0)).unwrap_or_else(|| "".to_string());
        let _ask_count = asks.get(i).map(|a| a.order_count.to_string()).unwrap_or_else(|| "".to_string());

        let row = Row::new(vec![
            Cell::from(bid_price).style(Style::default().fg(Color::Green)),
            Cell::from(bid_size).style(Style::default().fg(Color::Green)),
            Cell::from(bid_count).style(Style::default().fg(Color::Green)),
            Cell::from(" | ").style(Style::default().fg(Color::Gray)),
            Cell::from(ask_size).style(Style::default().fg(Color::Red)),
            Cell::from(ask_price).style(Style::default().fg(Color::Red)),
        ]);
        rows.push(row);
    }

    if rows.is_empty() {
        rows.push(Row::new(vec![
            Cell::from("").style(Style::default().fg(Color::Gray)),
            Cell::from("No bids").style(Style::default().fg(Color::Gray)),
            Cell::from("").style(Style::default().fg(Color::Gray)),
            Cell::from(" | ").style(Style::default().fg(Color::Gray)),
            Cell::from("No asks").style(Style::default().fg(Color::Gray)),
            Cell::from("").style(Style::default().fg(Color::Gray)),
        ]));
        rows.push(Row::new(vec![
            Cell::from("").style(Style::default().fg(Color::Gray)),
            Cell::from("").style(Style::default().fg(Color::Gray)),
            Cell::from("").style(Style::default().fg(Color::Gray)),
            Cell::from(" | ").style(Style::default().fg(Color::Gray)),
            Cell::from("").style(Style::default().fg(Color::Gray)),
            Cell::from("").style(Style::default().fg(Color::Gray)),
        ]));
        rows.push(Row::new(vec![
            Cell::from("").style(Style::default().fg(Color::Yellow)),
            Cell::from("Waiting for orders...").style(Style::default().fg(Color::Yellow)),
            Cell::from("").style(Style::default().fg(Color::Yellow)),
            Cell::from(" | ").style(Style::default().fg(Color::Gray)),
            Cell::from("Waiting for orders...").style(Style::default().fg(Color::Yellow)),
            Cell::from("").style(Style::default().fg(Color::Yellow)),
        ]));
    }

    rows
}


fn render_status_bar(f: &mut Frame, area: Rect) {
    let status_text = vec![
        Line::from(vec![
            Span::styled("NEAR Orderbook TUI", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
            Span::raw(" | "),
            Span::styled("Controls: ", Style::default().fg(Color::Yellow)),
            Span::raw("q=quit | r=refresh logs | c=clear orderbook | h=help"),
        ]),
        Line::from(vec![
            Span::styled("Status: ", Style::default().fg(Color::Green)),
            Span::raw("Live updates enabled | Press ESC or Q to exit"),
        ]),
    ];

    let status_bar = Paragraph::new(status_text)
        .block(Block::default().borders(Borders::ALL))
        .alignment(Alignment::Center);
    f.render_widget(status_bar, area);
}

fn read_solver_logs(solver_logs: &mut VecDeque<SolverLogEntry>) {
    // Try to read solver daemon logs from the expected location
    let solver_log_paths = vec![
        "../logs/solver.log",
        "logs/solver.log",
        "./logs/solver.log"
    ];

    for log_path in solver_log_paths {
        if let Ok(content) = std::fs::read_to_string(log_path) {
            // Get the last few lines (recent logs)
            let lines: Vec<&str> = content.lines().collect();
            let recent_lines = lines.iter().rev().take(20).rev(); // Last 20 lines

            for line in recent_lines {
                if !line.trim().is_empty() {
                    // Skip only the most verbose debug output, keep important processing info
                    if line.contains("Function execution return value") ||
                       line.contains("Here is your console command") ||
                       line.contains("[33m/Users/") ||
                       line.contains("Public key:") ||
                       line.contains("Signature:") ||
                       line.contains("Gas burned:") ||
                       line.contains("Transaction fee:") ||
                       line.contains("🔍 Raw NEAR CLI output:") ||
                       line.contains("🔍 Extracted JSON string:") ||
                       line.starts_with("[") && line.contains("]") && line.len() < 10 {
                        continue; // Skip verbose debug output but keep processing info
                    }

                    let level = if line.contains("ERROR") || line.contains("❌") {
                        "ERROR"
                    } else if line.contains("WARN") || line.contains("⚠️") {
                        "WARN"
                    } else if line.contains("INFO") || line.contains("✅") || line.contains("📤") || line.contains("🤖") || line.contains("🎯") || line.contains("⚙️") {
                        "INFO"
                    } else {
                        continue; // Skip all debug logs
                    };

                    // Check if this log entry is already present (avoid duplicates)
                    if !solver_logs.iter().any(|entry| entry.message == *line) {
                        let entry = SolverLogEntry {
                            timestamp: Instant::now(),
                            level: level.to_string(),
                            message: line.to_string(),
                            source: "solver-daemon".to_string(),
                        };

                        if solver_logs.len() >= 500 {
                            solver_logs.pop_front();
                        }
                        solver_logs.push_back(entry);
                    }
                }
            }
            break; // Found and processed logs, no need to check other paths
        }
    }
}

impl OrderbookAnimator {
    pub fn update_orderbook(&mut self, new_orderbook: &OrderbookData) {
        let now = Instant::now();
        let animation_duration = Duration::from_millis(1500); // 1.5s animations - longer so asks are visible
        let trade_flash_duration = Duration::from_millis(800); // 800ms trade flash

        // Compare with last snapshot to detect changes
        if let Some(last) = self.last_snapshot.clone() {
            // Detect removed orders (now disappearing)
            self.detect_removed_orders(&last.bids, &new_orderbook.bids, OrderSide::Bid, now, animation_duration);
            self.detect_removed_orders(&last.asks, &new_orderbook.asks, OrderSide::Ask, now, animation_duration);

            // Detect trades (price levels that had orders but now have different sizes)
            self.detect_trades(&last.bids, &new_orderbook.bids, OrderSide::Bid, now, trade_flash_duration);
            self.detect_trades(&last.asks, &new_orderbook.asks, OrderSide::Ask, now, trade_flash_duration);
        }

        // Detect new orders (now appearing)
        self.detect_new_orders(&new_orderbook.bids, OrderSide::Bid, now, animation_duration);
        self.detect_new_orders(&new_orderbook.asks, OrderSide::Ask, now, animation_duration);

        // Detect recent trades and create ghost orders for better visibility
        self.detect_recent_trades(now, animation_duration);

        // Update last snapshot
        self.last_snapshot = Some(new_orderbook.clone());
    }

    fn detect_removed_orders(&mut self, old_orders: &[PriceLevel], new_orders: &[PriceLevel], side: OrderSide, now: Instant, duration: Duration) {
        for old_order in old_orders {
            let price_key = format!("{:?}_{}", side, old_order.price);

            // Check if this price level no longer exists
            if !new_orders.iter().any(|new_order| new_order.price == old_order.price) {
                // Start disappearing animation
                let animated_order = AnimatedOrder {
                    price_level: old_order.clone(),
                    side: side.clone(),
                    animation_state: AnimationState::Disappearing { progress: 1.0 },
                    created_at: now,
                    expires_at: now + duration,
                };
                self.animated_orders.insert(price_key, animated_order);
            }
        }
    }

    fn detect_new_orders(&mut self, new_orders: &[PriceLevel], side: OrderSide, now: Instant, duration: Duration) {
        for new_order in new_orders {
            let price_key = format!("{:?}_{}", side, new_order.price);

            // Check if this is a new price level (not in last snapshot and not already animating)
            let is_new = if let Some(ref last) = self.last_snapshot {
                let old_orders = match side {
                    OrderSide::Bid => &last.bids,
                    OrderSide::Ask => &last.asks,
                };
                !old_orders.iter().any(|old_order| old_order.price == new_order.price)
            } else {
                true // First snapshot, all orders are new
            };

            if is_new && !self.animated_orders.contains_key(&price_key) {
                // Start appearing animation
                let animated_order = AnimatedOrder {
                    price_level: new_order.clone(),
                    side: side.clone(),
                    animation_state: AnimationState::Appearing { progress: 0.0 },
                    created_at: now,
                    expires_at: now + duration,
                };
                self.animated_orders.insert(price_key, animated_order);
            }
        }
    }

    fn detect_trades(&mut self, old_orders: &[PriceLevel], new_orders: &[PriceLevel], side: OrderSide, now: Instant, duration: Duration) {
        for old_order in old_orders {
            if let Some(new_order) = new_orders.iter().find(|new| new.price == old_order.price) {
                // Same price level exists but size changed - potential trade
                if new_order.size < old_order.size {
                    // Size decreased, likely a trade occurred
                    self.trade_flash = Some(TradeFlash {
                        price: old_order.price,
                        size: (old_order.size - new_order.size) as u64,
                        side: side.clone(),
                        created_at: now,
                        expires_at: now + duration,
                    });
                }
            } else {
                // Price level completely disappeared - trade consumed entire level
                self.trade_flash = Some(TradeFlash {
                    price: old_order.price,
                    size: old_order.size as u64,
                    side: side.clone(),
                    created_at: now,
                    expires_at: now + duration,
                });
            }
        }
    }

    fn detect_recent_trades(&mut self, now: Instant, duration: Duration) {
        // Parse solver logs to detect recent order submissions and create temporary orders
        if let Ok(content) = std::fs::read_to_string("logs/solver.log") {
            let lines: Vec<&str> = content.lines().collect();

            // Look for recent order submissions to show what briefly existed
            for (i, line) in lines.iter().enumerate() {
                if line.contains("📤 Submitting order to orderbook:") {
                    // Check if this was followed by a trade settlement
                    let following_lines = lines.iter().skip(i + 1).take(10);
                    let has_settlement = following_lines.clone().any(|l| l.contains("✅ Trade settled by orderbook:"));

                    if has_settlement {
                        // Parse order details to create ghost order showing what was matched
                        if let Some(price_match) = line.split("\"price\": ").nth(1) {
                            if let Some(price_str) = price_match.split(',').next() {
                                if let Ok(price) = price_str.trim().parse::<u64>() {
                                    if let Some(outcome_match) = line.split("\"outcome\": ").nth(1) {
                                        if let Some(outcome_str) = outcome_match.split(',').next() {
                                            if let Ok(outcome) = outcome_str.trim().parse::<u8>() {
                                                // For the order that got matched, show both the bid and ask that briefly existed

                                                // Show the order itself as a brief bid/ask
                                                let order_side = if outcome == 1 { OrderSide::Bid } else { OrderSide::Ask };
                                                let order_price = price;

                                                // Show the complement that would have existed momentarily
                                                let complement_side = if outcome == 1 { OrderSide::Ask } else { OrderSide::Bid };
                                                let complement_price = 100000 - price;

                                                // Only add if we haven't already created a ghost for this trade
                                                let trade_key = format!("{}_{}", order_price, complement_price);
                                                if !self.ghost_orders.iter().any(|g| {
                                                    (g.price == order_price && g.side == order_side) ||
                                                    (g.price == complement_price && g.side == complement_side)
                                                }) {
                                                    // Add ghost for the original order
                                                    self.ghost_orders.push(GhostOrder {
                                                        price: order_price,
                                                        size: 1000000,
                                                        side: order_side,
                                                        created_at: now,
                                                        expires_at: now + duration,
                                                        was_consumed: true,
                                                    });

                                                    // Add ghost for the complement (the ask that was briefly created)
                                                    self.ghost_orders.push(GhostOrder {
                                                        price: complement_price,
                                                        size: 1000000,
                                                        side: complement_side,
                                                        created_at: now,
                                                        expires_at: now + duration,
                                                        was_consumed: true,
                                                    });
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    pub fn update_animations(&mut self) {
        let now = Instant::now();
        let mut to_remove = Vec::new();

        // Update animation states
        for (key, animated) in &mut self.animated_orders {
            match animated.animation_state {
                AnimationState::Appearing { ref mut progress } => {
                    let elapsed = now.duration_since(animated.created_at);
                    let total_duration = animated.expires_at.duration_since(animated.created_at);

                    *progress = (elapsed.as_millis() as f32 / total_duration.as_millis() as f32).min(1.0);

                    if *progress >= 1.0 {
                        animated.animation_state = AnimationState::Stable;
                    }
                }
                AnimationState::Disappearing { ref mut progress } => {
                    let elapsed = now.duration_since(animated.created_at);
                    let total_duration = animated.expires_at.duration_since(animated.created_at);

                    *progress = 1.0 - (elapsed.as_millis() as f32 / total_duration.as_millis() as f32).max(0.0);

                    if *progress <= 0.0 || now >= animated.expires_at {
                        to_remove.push(key.clone());
                    }
                }
                AnimationState::Stable => {
                    // Stable orders don't need updates unless they become stale
                    // Keep stable animations for longer to preserve orderbook state
                    if now.duration_since(animated.created_at) > Duration::from_secs(30) {
                        to_remove.push(key.clone());
                    }
                }
            }
        }

        // Remove expired animations
        for key in to_remove {
            self.animated_orders.remove(&key);
        }

        // Update trade flash
        if let Some(ref flash) = self.trade_flash {
            if now >= flash.expires_at {
                self.trade_flash = None;
            }
        }

        // Clean up expired ghost orders
        self.ghost_orders.retain(|ghost| now < ghost.expires_at);
    }
}
