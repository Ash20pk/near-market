# 🚀 Supabase PostgreSQL Integration Setup

This guide will integrate your NEAR Prediction Marketplace orderbook with Supabase PostgreSQL for persistent order storage and real-time updates.

## 📋 Prerequisites

1. **Supabase Account**: [Create account](https://supabase.com) if needed
2. **Project Created**: Set up a new Supabase project
3. **Database URL**: Copy from Supabase project settings

## 🛠️ Database Setup

### Step 1: Apply Main Schema

Run the schema file in your Supabase SQL editor:

```bash
# Copy the schema
cat supabase-schema.sql
```

Or upload `supabase-schema.sql` to Supabase Dashboard → SQL Editor → New Query → Run

### Step 1.5: Apply Performance Indexes

**Important**: Run these indexes separately (one at a time) after the main schema:

1. Open Supabase Dashboard → SQL Editor → New Query
2. Run each index command from `supabase-indexes.sql` individually:

```sql
-- Run this first:
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_orders_market_side_price
ON orders (market_id, side, price)
WHERE status IN ('Pending', 'PartiallyFilled');
```

```sql
-- Then this (new query):
CREATE INDEX CONCURRENTLY IF NOT EXISTS idx_trades_market_time
ON trades (market_id, executed_at DESC);
```

Continue with each index from the file separately.

### Step 2: Verify Tables Created

Check that these tables exist:
- `orders` - Persistent orderbook orders
- `trades` - Trade execution history
- `collateral_balances` - User USDC balances
- `collateral_reservations` - Order collateral locks
- `market_stats` - Real-time market data (fixes N/A values!)
- `settlement_batches` - Batch settlement tracking

### Step 3: Test Market Stats Function

```sql
-- Test the stats update function
SELECT update_market_stats('test_market', 1);

-- Verify it works
SELECT * FROM market_stats WHERE market_id = 'test_market';
```

## ⚙️ Rust Integration

### Step 1: Add Dependencies

Add to `orderbook-service/Cargo.toml`:

```toml
[dependencies]
# Existing dependencies...
sqlx = { version = "0.7", features = ["runtime-tokio-rustls", "postgres", "uuid", "chrono", "bigdecimal"] }
tokio-postgres = "0.7"
deadpool-postgres = "0.10"
serde_json = "1.0"
```

### Step 2: Environment Variables

Add to `.env`:

```env
# Supabase Database
DATABASE_URL=postgresql://postgres:your_password@db.your_project.supabase.co:5432/postgres
SUPABASE_URL=https://your_project.supabase.co
SUPABASE_ANON_KEY=your_anon_key
SUPABASE_SERVICE_KEY=your_service_role_key

# Connection Pool
DATABASE_MAX_CONNECTIONS=10
DATABASE_MIN_CONNECTIONS=5
```

## 🔄 Migration Strategy

### Option A: Gradual Migration (Recommended)

1. **Keep existing in-memory storage** as fallback
2. **Add PostgreSQL writes** for all new orders/trades
3. **Read from PostgreSQL** for orderbook display
4. **Remove in-memory** once stable

### Option B: Complete Replacement

1. **Replace Database struct** entirely with PostgreSQL
2. **Update all storage calls** to use SQL queries
3. **Test thoroughly** before deployment

## 📊 Benefits You'll Get

### 1. **Persistent Ask Orders**
- ✅ Orders stay visible until filled/cancelled
- ✅ No more disappearing asks
- ✅ Real orderbook depth

### 2. **Accurate Market Stats**
- ✅ Real bid/ask/spread calculations
- ✅ No more N/A values in TUI
- ✅ Historical volume and trade data

### 3. **Real-time Updates**
- ✅ Supabase real-time subscriptions
- ✅ Live orderbook changes
- ✅ Instant trade notifications

### 4. **Production Ready**
- ✅ ACID transactions
- ✅ Concurrent user support
- ✅ Backup and recovery
- ✅ Scalable architecture

## 🎯 Key Files to Modify

1. `src/storage/mod.rs` - Replace with PostgreSQL implementation
2. `src/matching/mod.rs` - Update to use persistent storage
3. `src/ui.rs` - Read market stats from database
4. `src/main.rs` - Add database connection setup

## 🔧 Testing Plan

1. **Schema Validation**: Verify all tables and indexes
2. **Performance Testing**: Query speed with large datasets
3. **Real-time Testing**: Supabase subscriptions working
4. **Integration Testing**: Full orderbook flow
5. **Load Testing**: Multiple concurrent orders

## 📈 Expected Performance Improvements

- **Ask Visibility**: 100% (persistent storage)
- **Market Stats**: Real data instead of N/A
- **Order Matching**: Faster with proper indexes
- **Scalability**: Support 100+ concurrent users
- **Reliability**: No data loss on restart

## 🚨 Important Notes

1. **Backup existing data** before migration
2. **Test thoroughly** with small amounts first
3. **Monitor database performance** after deployment
4. **Set up proper monitoring** for production use

## 🔗 Next Steps

1. Apply schema to Supabase
2. Modify Rust code for PostgreSQL integration
3. Test with existing test suite
4. Deploy and monitor

Would you like me to help with any specific part of the integration?