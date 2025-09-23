#!/usr/bin/env node

// Simple PostgreSQL query to check recent trades
const { Client } = require('pg');

async function checkTrades() {
    const client = new Client({
        connectionString: process.env.DATABASE_URL || 'postgresql://postgres.fyusqrltfqpffxbfnbhs:X5R86zztCz9d2V6s@aws-1-ap-southeast-1.pooler.supabase.com:6543/postgres'
    });

    try {
        await client.connect();
        console.log('✅ Connected to PostgreSQL database');

        // Check total counts
        const ordersResult = await client.query('SELECT COUNT(*) as total_orders FROM orders');
        const tradesResult = await client.query('SELECT COUNT(*) as total_trades FROM trades');

        console.log(`📊 Database summary:`);
        console.log(`   Total orders: ${ordersResult.rows[0].total_orders}`);
        console.log(`   Total trades: ${tradesResult.rows[0].total_trades}`);

        // Check recent trades (last 24 hours)
        const recentTrades = await client.query(`
            SELECT trade_id, market_id, maker_account, taker_account, price, size,
                   settlement_status, executed_at
            FROM trades
            WHERE executed_at > NOW() - INTERVAL '24 hours'
            ORDER BY executed_at DESC
            LIMIT 10
        `);

        if (recentTrades.rows.length > 0) {
            console.log(`\n💰 Recent trades (last 24 hours):`);
            recentTrades.rows.forEach((trade, i) => {
                console.log(`   ${i + 1}. Trade ${trade.trade_id.substring(0, 8)}... `);
                console.log(`      Market: ${trade.market_id}`);
                console.log(`      Maker: ${trade.maker_account} → Taker: ${trade.taker_account}`);
                console.log(`      Price: $${(trade.price / 10000).toFixed(2)}, Size: ${trade.size}`);
                console.log(`      Status: ${trade.settlement_status}`);
                console.log(`      Time: ${trade.executed_at}`);
                console.log('');
            });
        } else {
            console.log('\n⚠️  No recent trades found in the last 24 hours');
        }

        // Check for ashpk20.testnet related trades
        const ashpkTrades = await client.query(`
            SELECT trade_id, market_id, maker_account, taker_account, price, size,
                   settlement_status, executed_at
            FROM trades
            WHERE maker_account LIKE '%ashpk20.testnet%'
               OR taker_account LIKE '%ashpk20.testnet%'
            ORDER BY executed_at DESC
            LIMIT 5
        `);

        if (ashpkTrades.rows.length > 0) {
            console.log(`🎯 ashpk20.testnet related trades:`);
            ashpkTrades.rows.forEach((trade, i) => {
                console.log(`   ${i + 1}. ${trade.trade_id.substring(0, 8)}... - ${trade.market_id}`);
                console.log(`      ${trade.maker_account} → ${trade.taker_account}`);
                console.log(`      $${(trade.price / 10000).toFixed(2)} x ${trade.size} (${trade.settlement_status})`);
                console.log(`      ${trade.executed_at}`);
            });
        } else {
            console.log('\n📭 No trades found for ashpk20.testnet');
        }

    } catch (error) {
        console.error('❌ Database error:', error.message);
    } finally {
        await client.end();
    }
}

checkTrades().catch(console.error);