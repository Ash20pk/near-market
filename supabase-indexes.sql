-- ================================
-- CONCURRENT INDEXES FOR PERFORMANCE
-- Run these AFTER the main schema is applied
-- These must be run outside of a transaction block
-- ================================

-- Performance index for orderbook queries (market + side + price filtering)
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_orders_market_side_price
ON orders (market_id, side, price)
WHERE status IN ('Pending', 'PartiallyFilled');

-- Time-based index for trade history queries
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_trades_market_time
ON trades (market_id, executed_at DESC);

-- Additional performance indexes for complex queries
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_orders_outcome_price
ON orders (market_id, outcome, price)
WHERE status IN ('Pending', 'PartiallyFilled');

-- Index for user order lookups
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_orders_user_status
ON orders (user_account, status, created_at DESC);

-- Index for settlement status queries
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_trades_settlement
ON trades (settlement_status, executed_at)
WHERE settlement_status != 'Settled';

-- Composite index for collateral balance lookups
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_collateral_market_balance
ON collateral_balances (market_id, available_balance DESC);

-- ================================
-- USAGE INSTRUCTIONS:
-- ================================
--
-- 1. First run: supabase-schema.sql (main schema)
-- 2. Then run these indexes one by one in Supabase SQL editor:
--
-- CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_orders_market_side_price
-- ON orders (market_id, side, price)
-- WHERE status IN ('Pending', 'PartiallyFilled');
--
-- CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_trades_market_time
-- ON trades (market_id, executed_at DESC);
--
-- etc.
--
-- Note: Each CREATE INDEX CONCURRENTLY must be run as a separate query
-- ================================