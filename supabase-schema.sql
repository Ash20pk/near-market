-- ================================
-- NEAR Prediction Marketplace Orderbook Database Schema
-- Verified against actual Rust codebase implementation
-- ================================

-- Enable UUID extension
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- ================================
-- ORDERS TABLE (Matches Order struct exactly)
-- ================================
CREATE TABLE orders (
    order_id UUID PRIMARY KEY,
    market_id TEXT NOT NULL,
    condition_id TEXT NOT NULL,
    user_account TEXT NOT NULL,
    outcome SMALLINT NOT NULL,            -- u8: 0=NO, 1=YES
    side TEXT NOT NULL,                   -- 'Buy' or 'Sell'
    order_type TEXT NOT NULL,             -- 'Limit' or 'Market'
    price BIGINT NOT NULL,                -- u64: basis points (5000 = $0.50)
    original_size NUMERIC(39,0) NOT NULL, -- u128: large integer support
    remaining_size NUMERIC(39,0) NOT NULL,
    filled_size NUMERIC(39,0) NOT NULL DEFAULT 0,
    status TEXT NOT NULL DEFAULT 'Pending', -- 'Pending', 'PartiallyFilled', 'Filled', 'Cancelled', 'Expired', 'Failed'
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    solver_account TEXT NOT NULL
);

-- Indexes for orders table
CREATE INDEX idx_orders_market_outcome_side_status ON orders (market_id, outcome, side, status);
CREATE INDEX idx_orders_active ON orders (market_id, outcome, side, price) WHERE status IN ('Pending', 'PartiallyFilled');
CREATE INDEX idx_orders_user ON orders (user_account);
CREATE INDEX idx_orders_created ON orders (created_at DESC);
CREATE INDEX idx_orders_expires ON orders (expires_at) WHERE expires_at IS NOT NULL;

-- ================================
-- TRADES TABLE (Matches Trade struct exactly)
-- ================================
CREATE TABLE trades (
    trade_id UUID PRIMARY KEY,
    market_id TEXT NOT NULL,
    condition_id TEXT NOT NULL,
    maker_order_id UUID NOT NULL,
    taker_order_id UUID NOT NULL,
    maker_account TEXT NOT NULL,
    taker_account TEXT NOT NULL,
    maker_side TEXT NOT NULL,             -- 'Buy' or 'Sell'
    taker_side TEXT NOT NULL,             -- 'Buy' or 'Sell'
    outcome SMALLINT NOT NULL,            -- u8: which outcome was traded
    price BIGINT NOT NULL,                -- u64: execution price in basis points
    size NUMERIC(39,0) NOT NULL,          -- u128: trade size
    trade_type TEXT NOT NULL,             -- 'DirectMatch', 'Minting', 'Burning'
    executed_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    settlement_status TEXT NOT NULL DEFAULT 'Pending', -- 'Pending', 'Settling', 'Settled', 'Failed'
    settlement_tx_hash TEXT
);

-- Add foreign key constraints
ALTER TABLE trades ADD CONSTRAINT fk_trades_maker_order
    FOREIGN KEY (maker_order_id) REFERENCES orders(order_id);
ALTER TABLE trades ADD CONSTRAINT fk_trades_taker_order
    FOREIGN KEY (taker_order_id) REFERENCES orders(order_id);

-- Indexes for trades table
CREATE INDEX idx_trades_market_outcome ON trades (market_id, outcome);
CREATE INDEX idx_trades_settlement_status ON trades (settlement_status);
CREATE INDEX idx_trades_executed ON trades (executed_at DESC);
CREATE INDEX idx_trades_accounts ON trades (maker_account, taker_account);
CREATE INDEX idx_trades_pending ON trades (settlement_status) WHERE settlement_status = 'Pending';

-- ================================
-- COLLATERAL BALANCES (Polymarket-style)
-- ================================
CREATE TABLE collateral_balances (
    account_id TEXT NOT NULL,
    market_id TEXT NOT NULL,
    available_balance NUMERIC(39,0) NOT NULL DEFAULT 0,  -- Free USDC
    reserved_balance NUMERIC(39,0) NOT NULL DEFAULT 0,   -- Reserved for orders
    position_balance NUMERIC(39,0) NOT NULL DEFAULT 0,   -- Token value
    total_deposited NUMERIC(39,0) NOT NULL DEFAULT 0,
    total_withdrawn NUMERIC(39,0) NOT NULL DEFAULT 0,
    last_updated TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    PRIMARY KEY (account_id, market_id)
);

-- Indexes for collateral balances
CREATE INDEX idx_collateral_account ON collateral_balances (account_id);
CREATE INDEX idx_collateral_updated ON collateral_balances (last_updated DESC);

-- ================================
-- COLLATERAL RESERVATIONS
-- ================================
CREATE TABLE collateral_reservations (
    order_id UUID PRIMARY KEY,
    reservation_id UUID NOT NULL DEFAULT uuid_generate_v4(),
    account_id TEXT NOT NULL,
    market_id TEXT NOT NULL,
    reserved_amount NUMERIC(39,0) NOT NULL,  -- USDC reserved
    max_loss NUMERIC(39,0) NOT NULL,         -- Maximum possible loss
    side TEXT NOT NULL,                      -- 'Buy' or 'Sell'
    price BIGINT NOT NULL,                   -- Order price in basis points
    size NUMERIC(39,0) NOT NULL,             -- Order size
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Foreign key and indexes for reservations
ALTER TABLE collateral_reservations ADD CONSTRAINT fk_reservations_order
    FOREIGN KEY (order_id) REFERENCES orders(order_id) ON DELETE CASCADE;
CREATE INDEX idx_reservations_account_market ON collateral_reservations (account_id, market_id);

-- ================================
-- MARKET STATS (For TUI display - fixes N/A values!)
-- ================================
CREATE TABLE market_stats (
    market_id TEXT NOT NULL,
    outcome SMALLINT NOT NULL,
    last_price BIGINT,                    -- Last trade price
    best_bid BIGINT,                      -- Highest buy price
    best_ask BIGINT,                      -- Lowest sell price
    bid_volume NUMERIC(39,0) DEFAULT 0,   -- Total buy volume
    ask_volume NUMERIC(39,0) DEFAULT 0,   -- Total sell volume
    total_volume NUMERIC(39,0) DEFAULT 0, -- All-time volume
    trade_count INTEGER DEFAULT 0,        -- Number of trades
    spread BIGINT,                        -- best_ask - best_bid
    mid_price BIGINT,                     -- (best_bid + best_ask) / 2
    updated_at TIMESTAMPTZ DEFAULT NOW(),

    PRIMARY KEY (market_id, outcome)
);

-- Indexes for market stats
CREATE INDEX idx_market_stats_updated ON market_stats (updated_at DESC);

-- ================================
-- SETTLEMENT BATCHES (For efficient on-chain execution)
-- ================================
CREATE TABLE settlement_batches (
    batch_id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    total_gas_estimate BIGINT NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    submitted_at TIMESTAMPTZ,
    completed_at TIMESTAMPTZ,
    status TEXT NOT NULL DEFAULT 'Pending', -- 'Pending', 'Submitted', 'Completed', 'Failed'
    tx_hash TEXT
);

-- Indexes for settlement batches
CREATE INDEX idx_batches_status ON settlement_batches (status);
CREATE INDEX idx_batches_created ON settlement_batches (created_at DESC);

-- Trades in each batch
CREATE TABLE batch_trades (
    batch_id UUID NOT NULL,
    trade_id UUID NOT NULL,
    PRIMARY KEY (batch_id, trade_id)
);

-- Foreign keys for batch trades
ALTER TABLE batch_trades ADD CONSTRAINT fk_batch_trades_batch
    FOREIGN KEY (batch_id) REFERENCES settlement_batches(batch_id);
ALTER TABLE batch_trades ADD CONSTRAINT fk_batch_trades_trade
    FOREIGN KEY (trade_id) REFERENCES trades(trade_id);

-- ================================
-- FUNCTIONS FOR MARKET STATS UPDATES
-- ================================

-- Function to update market stats after order/trade changes
CREATE OR REPLACE FUNCTION update_market_stats(p_market_id TEXT, p_outcome SMALLINT)
RETURNS VOID AS $$
DECLARE
    v_best_bid BIGINT;
    v_best_ask BIGINT;
    v_last_price BIGINT;
    v_bid_volume NUMERIC(39,0);
    v_ask_volume NUMERIC(39,0);
    v_total_volume NUMERIC(39,0);
    v_trade_count INTEGER;
    v_spread BIGINT;
    v_mid_price BIGINT;
BEGIN
    -- Calculate best bid (highest buy price)
    SELECT MAX(price) INTO v_best_bid
    FROM orders
    WHERE market_id = p_market_id
      AND outcome = p_outcome
      AND side = 'Buy'
      AND status IN ('Pending', 'PartiallyFilled');

    -- Calculate best ask (lowest sell price)
    SELECT MIN(price) INTO v_best_ask
    FROM orders
    WHERE market_id = p_market_id
      AND outcome = p_outcome
      AND side = 'Sell'
      AND status IN ('Pending', 'PartiallyFilled');

    -- Get last trade price
    SELECT price INTO v_last_price
    FROM trades
    WHERE market_id = p_market_id
      AND outcome = p_outcome
    ORDER BY executed_at DESC
    LIMIT 1;

    -- Calculate bid volume
    SELECT COALESCE(SUM(remaining_size), 0) INTO v_bid_volume
    FROM orders
    WHERE market_id = p_market_id
      AND outcome = p_outcome
      AND side = 'Buy'
      AND status IN ('Pending', 'PartiallyFilled');

    -- Calculate ask volume
    SELECT COALESCE(SUM(remaining_size), 0) INTO v_ask_volume
    FROM orders
    WHERE market_id = p_market_id
      AND outcome = p_outcome
      AND side = 'Sell'
      AND status IN ('Pending', 'PartiallyFilled');

    -- Calculate total volume and trade count
    SELECT COALESCE(SUM(size), 0), COUNT(*) INTO v_total_volume, v_trade_count
    FROM trades
    WHERE market_id = p_market_id AND outcome = p_outcome;

    -- Calculate spread and mid price
    IF v_best_bid IS NOT NULL AND v_best_ask IS NOT NULL THEN
        v_spread := v_best_ask - v_best_bid;
        v_mid_price := (v_best_bid + v_best_ask) / 2;
    END IF;

    -- Insert or update market stats
    INSERT INTO market_stats (
        market_id, outcome, last_price, best_bid, best_ask,
        bid_volume, ask_volume, total_volume, trade_count,
        spread, mid_price, updated_at
    ) VALUES (
        p_market_id, p_outcome, v_last_price, v_best_bid, v_best_ask,
        v_bid_volume, v_ask_volume, v_total_volume, v_trade_count,
        v_spread, v_mid_price, NOW()
    )
    ON CONFLICT (market_id, outcome)
    DO UPDATE SET
        last_price = EXCLUDED.last_price,
        best_bid = EXCLUDED.best_bid,
        best_ask = EXCLUDED.best_ask,
        bid_volume = EXCLUDED.bid_volume,
        ask_volume = EXCLUDED.ask_volume,
        total_volume = EXCLUDED.total_volume,
        trade_count = EXCLUDED.trade_count,
        spread = EXCLUDED.spread,
        mid_price = EXCLUDED.mid_price,
        updated_at = NOW();
END;
$$ LANGUAGE plpgsql;

-- ================================
-- TRIGGERS FOR AUTOMATIC STATS UPDATES
-- ================================

-- Trigger function for order changes
CREATE OR REPLACE FUNCTION trigger_update_market_stats_orders()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM update_market_stats(COALESCE(NEW.market_id, OLD.market_id), COALESCE(NEW.outcome, OLD.outcome));
    RETURN COALESCE(NEW, OLD);
END;
$$ LANGUAGE plpgsql;

-- Trigger function for trade changes
CREATE OR REPLACE FUNCTION trigger_update_market_stats_trades()
RETURNS TRIGGER AS $$
BEGIN
    PERFORM update_market_stats(NEW.market_id, NEW.outcome);
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

-- Create triggers
CREATE TRIGGER trigger_orders_stats
    AFTER INSERT OR UPDATE OR DELETE ON orders
    FOR EACH ROW EXECUTE FUNCTION trigger_update_market_stats_orders();

CREATE TRIGGER trigger_trades_stats
    AFTER INSERT ON trades
    FOR EACH ROW EXECUTE FUNCTION trigger_update_market_stats_trades();

-- ================================
-- VIEWS FOR EASY ORDERBOOK QUERIES
-- ================================

-- View for current orderbook state
CREATE VIEW orderbook_view AS
SELECT
    market_id,
    outcome,
    side,
    price,
    SUM(remaining_size) as total_size,
    COUNT(*) as order_count
FROM orders
WHERE status IN ('Pending', 'PartiallyFilled')
GROUP BY market_id, outcome, side, price
ORDER BY market_id, outcome, side,
    CASE WHEN side = 'Buy' THEN price END DESC,
    CASE WHEN side = 'Sell' THEN price END ASC;

-- View for market overview
CREATE VIEW market_overview AS
SELECT
    ms.*,
    ROUND((ms.best_bid::DECIMAL / 100), 2) as bid_percent,
    ROUND((ms.best_ask::DECIMAL / 100), 2) as ask_percent,
    ROUND((ms.mid_price::DECIMAL / 100), 2) as mid_percent,
    ROUND((ms.last_price::DECIMAL / 100), 2) as last_percent
FROM market_stats ms;

-- ================================
-- RLS (Row Level Security) Setup for Multi-tenancy
-- ================================

-- Enable RLS on all tables
ALTER TABLE orders ENABLE ROW LEVEL SECURITY;
ALTER TABLE trades ENABLE ROW LEVEL SECURITY;
ALTER TABLE collateral_balances ENABLE ROW LEVEL SECURITY;
ALTER TABLE collateral_reservations ENABLE ROW LEVEL SECURITY;

-- Create policies (example - adjust based on your auth system)
-- CREATE POLICY "Users can view their own orders" ON orders
--     FOR SELECT USING (user_account = current_setting('app.current_user_account'));

-- CREATE POLICY "Users can view their own collateral" ON collateral_balances
--     FOR ALL USING (account_id = current_setting('app.current_user_account'));

-- Note: Market stats and settlement data should be publicly readable
ALTER TABLE market_stats DISABLE ROW LEVEL SECURITY;
ALTER TABLE settlement_batches DISABLE ROW LEVEL SECURITY;
ALTER TABLE batch_trades DISABLE ROW LEVEL SECURITY;

-- Comments for documentation
COMMENT ON TABLE orders IS 'Persistent orderbook orders matching Rust Order struct';
COMMENT ON TABLE trades IS 'Executed trades matching Rust Trade struct';
COMMENT ON TABLE market_stats IS 'Real-time market statistics for TUI display';
COMMENT ON FUNCTION update_market_stats IS 'Updates market stats after order/trade changes';