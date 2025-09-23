#!/usr/bin/env node

/**
 * Intent-Focused Test Script
 * Tests NEAR chain intent submission with simplified flow
 *
 * This script focuses on:
 * 1. One complementary order pair (YES@60% + NO@40%)
 * 2. Multiple realistic buy/sell intents submitted via NEAR
 * 3. Simplified flow without complex approval management
 */

const { connect, keyStores, KeyPair, utils } = require('near-api-js');
const fs = require('fs');
const os = require('os');
const path = require('path');

// Configuration using deployed contracts
const CONFIG = {
    network: 'testnet',
    contracts: {
        verifier: 'verifier.ashpk20.testnet',
        solver: 'solver.ashpk20.testnet',
        ctf: 'ctf.ashpk20.testnet',
        resolver: 'resolver.ashpk20.testnet',
        usdc: '3e2210e1184b45b64c8a434c0a7e7b23cc04ea7eb7a6c3c32520d03d4afcb8af',
        orderbook_service: 'ashpk20.testnet'
    },
    accounts: {
        master: 'ashpk20.testnet',
        market_maker: 'market-maker.ashpk20.testnet',
        trader1: 'trader-joe-1.ashpk20.testnet',
        trader2: 'trader-joe-2.ashpk20.testnet'
    },
    amounts: {
        tiny_order: "500000",         // 0.5 USDC - micro trades
        small_order: "1000000",       // 1.0 USDC - retail size
        medium_order: "2500000",      // 2.5 USDC - typical retail
        large_order: "5000000",       // 5.0 USDC - bigger retail
        whale_order: "8000000",       // 8.0 USDC - max realistic size
        usdc_balance: "9000000"       // 9 USDC total balance per trader (<$10 as requested)
    }
};

class IntentTester {
    constructor() {
        this.marketId = null;
        this.conditionId = null;
        this.near = null;
        this.accounts = {};
        this.loadLatestMarket();

        // NEAR API JS configuration
        this.nearConfig = {
            networkId: CONFIG.network,
            nodeUrl: 'https://billowing-ancient-meadow.near-testnet.quiknode.pro/02fe7cae1f78374077f55e172eba1f849e8570f4/',
            keyStore: new keyStores.UnencryptedFileSystemKeyStore(path.join(os.homedir(), '.near-credentials')),
        };
    }

    loadLatestMarket() {
        try {
            const latestMarketPath = path.join(__dirname, 'latest_market.json');
            if (fs.existsSync(latestMarketPath)) {
                const data = JSON.parse(fs.readFileSync(latestMarketPath, 'utf8'));
                this.marketId = data.latest_market_id;
                this.log(`📊 Using latest market: ${this.marketId}`);
            } else {
                throw new Error('Latest market file not found');
            }
        } catch (error) {
            this.log(`❌ Failed to load latest market: ${error.message}`);
            this.log('💡 Please register a market first');
            process.exit(1);
        }
    }

    log(message) {
        const timestamp = new Date().toISOString();
        console.log(`[${timestamp}] ${message}`);
    }

    async sleep(ms) {
        return new Promise(resolve => setTimeout(resolve, ms));
    }

    async initializeNear() {
        this.log('🔗 Connecting to NEAR...');
        this.near = await connect(this.nearConfig);

        // Load accounts
        for (const [name, accountId] of Object.entries(CONFIG.accounts)) {
            try {
                this.accounts[name] = await this.near.account(accountId);
                this.log(`✅ Connected to ${name}: ${accountId}`);
            } catch (error) {
                this.log(`❌ Failed to connect to ${name}: ${error.message}`);
            }
        }
    }

    async nearCall(contractId, methodName, args, signerAccountId, gas = '300000000000000', deposit = '0') {
        try {
            const account = this.accounts[Object.keys(CONFIG.accounts).find(key => CONFIG.accounts[key] === signerAccountId)];
            if (!account) {
                throw new Error(`Account ${signerAccountId} not found`);
            }

            this.log(`🔗 Calling ${contractId}.${methodName} from ${signerAccountId}`);

            const result = await account.functionCall({
                contractId,
                methodName,
                args,
                gas,
                attachedDeposit: deposit
            });

            this.log(`✅ Transaction successful: ${result.transaction.hash}`);
            return result;
        } catch (error) {
            this.log(`❌ NEAR call failed: ${error.message}`);
            throw error;
        }
    }

    async authorizeTrader(accountName, purpose) {
        this.log(`🔐 Setting up complete authorization for ${accountName} (${purpose})`);

        try {
            // Step 1: Register with USDC contract for storage
            await this.nearCall(
                CONFIG.contracts.usdc,
                'storage_deposit',
                {
                    account_id: CONFIG.accounts[accountName],
                    registration_only: true
                },
                CONFIG.accounts[accountName],
                '300000000000000',
                '1250000000000000000000' // Storage deposit
            );
            this.log(`   ✅ USDC storage registration`);

            // Step 2: Register CTF contract with USDC (so CTF can receive USDC)
            await this.nearCall(
                CONFIG.contracts.usdc,
                'storage_deposit',
                {
                    account_id: CONFIG.contracts.ctf,
                    registration_only: true
                },
                CONFIG.accounts[accountName],
                '300000000000000',
                '1250000000000000000000'
            );
            this.log(`   ✅ CTF contract USDC registration`);

            // Step 3: Approve CTF contract to spend USDC (for minting tokens)
            const approvalAmount = "100000000"; // 100 USDC approval
            await this.nearCall(
                CONFIG.contracts.usdc,
                'approve',
                {
                    spender_id: CONFIG.contracts.ctf,
                    value: approvalAmount
                },
                CONFIG.accounts[accountName],
                '300000000000000'
            );
            this.log(`   ✅ CTF contract USDC spending approval: ${approvalAmount / 1000000} USDC`);

            // Step 4: Approve orderbook service to spend USDC (for settlements)
            await this.nearCall(
                CONFIG.contracts.usdc,
                'approve',
                {
                    spender_id: CONFIG.contracts.orderbook_service,
                    value: approvalAmount
                },
                CONFIG.accounts[accountName],
                '300000000000000'
            );
            this.log(`   ✅ Orderbook service USDC spending approval: ${approvalAmount / 1000000} USDC`);

            // Step 5: Register for CTF token storage
            await this.nearCall(
                CONFIG.contracts.ctf,
                'set_approval_for_all',
                {
                    operator: CONFIG.contracts.orderbook_service,
                    approved: true
                },
                CONFIG.accounts[accountName],
                '300000000000000'
            );
            this.log(`   ✅ Orderbook CTF token transfer approval`);

            this.log(`✅ Complete authorization set for ${accountName}`);

        } catch (error) {
            this.log(`⚠️  Authorization failed (continuing anyway): ${error.message}`);
        }
    }

    async submitIntent(intent, accountName, description) {
        this.log(`🎯 Submitting intent: ${description}`);
        this.log(`   User: ${CONFIG.accounts[accountName]}`);
        this.log(`   Market: ${intent.market_id}`);
        this.log(`   Outcome: ${intent.outcome === 1 ? 'YES' : 'NO'} @ ${intent.max_price ? (intent.max_price / 1000).toFixed(1) + '¢' : intent.min_price ? (intent.min_price / 1000).toFixed(1) + '¢' : 'MARKET'}`);
        this.log(`   Size: $${(intent.amount / 1000000).toFixed(2)} USDC`);

        try {
            const result = await this.nearCall(
                CONFIG.contracts.verifier,
                'verify_and_solve',
                {
                    intent: intent,
                    solver_account: CONFIG.contracts.solver
                },
                CONFIG.accounts[accountName]
            );

            this.log(`✅ Intent submitted successfully`);
            return result;
        } catch (error) {
            this.log(`❌ Intent submission failed: ${error.message}`);
            throw error;
        }
    }

    async testComplementaryOrders() {
        this.log('\n🎯 TEST 1: Complementary Order Pair');
        this.log('💡 Submitting YES@60¢ + NO@40¢ orders for complementary matching');

        // Authorize traders to allow verifier/orderbook to handle their tokens
        await this.authorizeTrader('trader1', 'YES buy orders');
        await this.authorizeTrader('trader2', 'NO buy orders');

        // YES buy intent at 60%
        const yesIntent = {
            intent_id: `yes_comp_${Date.now()}_${Math.random()}`,
            user: CONFIG.accounts.trader1,
            market_id: this.marketId,
            intent_type: "BuyShares",
            outcome: 1, // YES
            amount: CONFIG.amounts.medium_order,
            max_price: 60000, // 60 cents ($0.60)
            min_price: null,
            deadline: (Date.now() + 24 * 60 * 60 * 1000) * 1000000,
            order_type: "GTC"
        };

        await this.submitIntent(yesIntent, 'trader1', 'YES buy @ 60¢');
        await this.sleep(3000);

        // NO buy intent at 40 cents
        const noIntent = {
            intent_id: `no_comp_${Date.now()}_${Math.random()}`,
            user: CONFIG.accounts.trader2,
            market_id: this.marketId,
            intent_type: "BuyShares",
            outcome: 0, // NO
            amount: CONFIG.amounts.medium_order,
            max_price: 40000, // 40 cents ($0.40)
            min_price: null,
            deadline: (Date.now() + 24 * 60 * 60 * 1000) * 1000000,
            order_type: "GTC"
        };

        await this.submitIntent(noIntent, 'trader2', 'NO buy @ 40¢');

        this.log('✅ Complementary order pair submitted');
        this.log('🎯 Expected: Automatic matching via minting (YES@60¢ + NO@40¢ = $1.00)');

        await this.sleep(5000); // Wait for processing
    }

    async testMultipleBuyOrders() {
        this.log('\n📈 TEST 2: Multiple Buy Orders');
        this.log('💡 Submitting various buy orders at different price levels');

        // Authorize market maker for token handling
        await this.authorizeTrader('market_maker', 'market making operations');

        const buyOrders = [
            {
                account: 'trader1',
                outcome: 1, // YES
                price: 58000, // 58 cents
                amount: CONFIG.amounts.tiny_order,
                desc: 'Micro YES buy @ 58¢',
                order_type: 'GTC'
            },
            {
                account: 'trader2',
                outcome: 1, // YES
                price: 62000, // 62 cents
                amount: CONFIG.amounts.small_order,
                desc: 'Small YES buy @ 62¢',
                order_type: 'GTC'
            },
            {
                account: 'trader1',
                outcome: 0, // NO
                price: 42000, // 42 cents
                amount: CONFIG.amounts.tiny_order,
                desc: 'Micro NO buy @ 42¢',
                order_type: 'GTC'
            },
            {
                account: 'trader2',
                outcome: 0, // NO
                price: 38000, // 38 cents
                amount: CONFIG.amounts.small_order,
                desc: 'Small NO buy @ 38¢',
                order_type: 'FOK' // Fill-or-Kill for variety
            },
            {
                account: 'market_maker',
                outcome: 1, // YES
                price: 65000, // 65 cents
                amount: CONFIG.amounts.medium_order,
                desc: 'Medium YES buy @ 65¢',
                order_type: 'GTD' // Good-Till-Date
            }
        ];

        for (const order of buyOrders) {
            const intent = {
                intent_id: `buy_${Date.now()}_${Math.random()}`,
                user: CONFIG.accounts[order.account],
                market_id: this.marketId,
                intent_type: "BuyShares",
                outcome: order.outcome,
                amount: order.amount,
                max_price: order.price,
                min_price: null,
                deadline: (Date.now() + 24 * 60 * 60 * 1000) * 1000000,
                order_type: order.order_type || "GTC"
            };

            try {
                await this.submitIntent(intent, order.account, order.desc);
                await this.sleep(2000);
            } catch (error) {
                this.log(`⚠️  ${order.desc} failed: ${error.message}`);
            }
        }
    }

    async testMultipleSellOrders() {
        this.log('\n📉 TEST 3: Multiple Sell Orders');
        this.log('💡 Submitting sell orders (requires existing token positions)');

        const sellOrders = [
            {
                account: 'trader1',
                outcome: 1, // Selling YES
                price: 68000, // 68 cents
                amount: CONFIG.amounts.tiny_order,
                desc: 'Micro YES sell @ 68¢',
                order_type: 'GTC'
            },
            {
                account: 'trader2',
                outcome: 0, // Selling NO
                price: 44000, // 44 cents
                amount: CONFIG.amounts.tiny_order,
                desc: 'Micro NO sell @ 44¢',
                order_type: 'FAK' // Fill-and-Kill
            },
            {
                account: 'market_maker',
                outcome: 1, // Selling YES
                price: 72000, // 72 cents
                amount: CONFIG.amounts.small_order,
                desc: 'Small YES sell @ 72¢',
                order_type: 'GTC'
            }
        ];

        for (const order of sellOrders) {
            const intent = {
                intent_id: `sell_${Date.now()}_${Math.random()}`,
                user: CONFIG.accounts[order.account],
                market_id: this.marketId,
                intent_type: "SellShares",
                outcome: order.outcome,
                amount: order.amount,
                min_price: order.price,
                max_price: null,
                deadline: (Date.now() + 24 * 60 * 60 * 1000) * 1000000,
                order_type: order.order_type || "GTC"
            };

            try {
                await this.submitIntent(intent, order.account, order.desc);
                await this.sleep(2000);
            } catch (error) {
                this.log(`⚠️  ${order.desc} failed: ${error.message}`);
            }
        }
    }

    async testMarketOrders() {
        this.log('\n⚡ TEST 4: Market Orders');
        this.log('💡 Testing immediate execution market orders');

        const marketOrders = [
            {
                account: 'trader1',
                outcome: 1, // Buy YES at market
                amount: CONFIG.amounts.tiny_order,
                type: 'BuyShares',
                desc: 'Market buy YES (micro)',
                order_type: 'Market'
            },
            {
                account: 'trader2',
                outcome: 0, // Buy NO at market
                amount: CONFIG.amounts.tiny_order,
                type: 'BuyShares',
                desc: 'Market buy NO (micro)',
                order_type: 'Market'
            }
        ];

        for (const order of marketOrders) {
            const intent = {
                intent_id: `market_${Date.now()}_${Math.random()}`,
                user: CONFIG.accounts[order.account],
                market_id: this.marketId,
                intent_type: order.type,
                outcome: order.outcome,
                amount: order.amount,
                max_price: null, // Market orders don't specify price
                min_price: null,
                deadline: (Date.now() + 24 * 60 * 60 * 1000) * 1000000,
                order_type: order.order_type || "Market"
            };

            try {
                await this.submitIntent(intent, order.account, order.desc);
                await this.sleep(2000);
            } catch (error) {
                this.log(`⚠️  ${order.desc} failed: ${error.message}`);
            }
        }
    }

    async testRealisticMarketMaking() {
        this.log('\n🏦 TEST 5: Realistic Market Making');
        this.log('💡 Creating proper bid/ask spreads like Polymarket');

        // Market making orders that create proper spreads
        const marketMakingOrders = [
            // YES token liquidity (around 60¢ fair value)
            { account: 'market_maker', outcome: 1, side: 'Buy', price: 55000, amount: CONFIG.amounts.medium_order, desc: 'MM YES bid @ 55¢', order_type: 'GTC' },
            { account: 'market_maker', outcome: 1, side: 'Buy', price: 58000, amount: CONFIG.amounts.small_order, desc: 'MM YES bid @ 58¢', order_type: 'GTC' },
            { account: 'market_maker', outcome: 1, side: 'Sell', price: 62000, amount: CONFIG.amounts.small_order, desc: 'MM YES ask @ 62¢', order_type: 'GTC' },
            { account: 'market_maker', outcome: 1, side: 'Sell', price: 65000, amount: CONFIG.amounts.medium_order, desc: 'MM YES ask @ 65¢', order_type: 'GTC' },

            // NO token liquidity (around 40¢ fair value)
            { account: 'market_maker', outcome: 0, side: 'Buy', price: 35000, amount: CONFIG.amounts.medium_order, desc: 'MM NO bid @ 35¢', order_type: 'GTC' },
            { account: 'market_maker', outcome: 0, side: 'Buy', price: 38000, amount: CONFIG.amounts.small_order, desc: 'MM NO bid @ 38¢', order_type: 'GTC' },
            { account: 'market_maker', outcome: 0, side: 'Sell', price: 42000, amount: CONFIG.amounts.small_order, desc: 'MM NO ask @ 42¢', order_type: 'GTC' },
            { account: 'market_maker', outcome: 0, side: 'Sell', price: 45000, amount: CONFIG.amounts.medium_order, desc: 'MM NO ask @ 45¢', order_type: 'GTC' },
        ];

        for (const order of marketMakingOrders) {
            const intent = {
                intent_id: `mm_${Date.now()}_${Math.random()}`,
                user: CONFIG.accounts[order.account],
                market_id: this.marketId,
                intent_type: order.side === 'Buy' ? 'BuyShares' : 'SellShares',
                outcome: order.outcome,
                amount: order.amount,
                max_price: order.side === 'Buy' ? order.price : null,
                min_price: order.side === 'Sell' ? order.price : null,
                deadline: (Date.now() + 24 * 60 * 60 * 1000) * 1000000,
                order_type: order.order_type
            };

            try {
                await this.submitIntent(intent, order.account, order.desc);
                await this.sleep(1500);
            } catch (error) {
                this.log(`⚠️  ${order.desc} failed: ${error.message}`);
            }
        }
    }

    async testRetailTradingPatterns() {
        this.log('\n🛒 TEST 6: Retail Trading Patterns');
        this.log('💡 Simulating realistic retail trader behavior with small sizes');

        const retailOrders = [
            // Bullish retail traders
            { account: 'trader1', outcome: 1, type: 'BuyShares', price: 61000, amount: CONFIG.amounts.tiny_order, desc: 'Retail YES buy @ 61¢', order_type: 'GTC' },
            { account: 'trader2', outcome: 1, type: 'BuyShares', price: 59000, amount: CONFIG.amounts.small_order, desc: 'Retail YES buy @ 59¢', order_type: 'GTC' },

            // Bearish retail traders
            { account: 'trader1', outcome: 0, type: 'BuyShares', price: 41000, amount: CONFIG.amounts.tiny_order, desc: 'Retail NO buy @ 41¢', order_type: 'GTC' },
            { account: 'trader2', outcome: 0, type: 'BuyShares', price: 39000, amount: CONFIG.amounts.small_order, desc: 'Retail NO buy @ 39¢', order_type: 'GTC' },

            // Market orders (FOMO trades)
            { account: 'trader1', outcome: 1, type: 'BuyShares', amount: CONFIG.amounts.tiny_order, desc: 'FOMO market buy YES', order_type: 'Market' },
            { account: 'trader2', outcome: 0, type: 'BuyShares', amount: CONFIG.amounts.tiny_order, desc: 'FOMO market buy NO', order_type: 'Market' },

            // Profit taking (requires existing positions)
            { account: 'trader1', outcome: 1, type: 'SellShares', price: 68000, amount: CONFIG.amounts.tiny_order, desc: 'Profit take YES @ 68¢', order_type: 'GTC' },
            { account: 'trader2', outcome: 0, type: 'SellShares', price: 48000, amount: CONFIG.amounts.tiny_order, desc: 'Profit take NO @ 48¢', order_type: 'GTC' },
        ];

        for (const order of retailOrders) {
            const intent = {
                intent_id: `retail_${Date.now()}_${Math.random()}`,
                user: CONFIG.accounts[order.account],
                market_id: this.marketId,
                intent_type: order.type,
                outcome: order.outcome,
                amount: order.amount,
                max_price: order.type === 'BuyShares' ? order.price : null,
                min_price: order.type === 'SellShares' ? order.price : null,
                deadline: (Date.now() + 24 * 60 * 60 * 1000) * 1000000,
                order_type: order.order_type
            };

            try {
                await this.submitIntent(intent, order.account, order.desc);
                await this.sleep(2000);
            } catch (error) {
                this.log(`⚠️  ${order.desc} failed: ${error.message}`);
            }
        }
    }

    async testAdvancedOrderTypes() {
        this.log('\n⚡ TEST 7: Advanced Order Types');
        this.log('💡 Testing FOK, FAK, and GTD order types like Polymarket');

        const advancedOrders = [
            // Fill-or-Kill (FOK) - must execute completely or cancel
            { account: 'trader1', outcome: 1, type: 'BuyShares', price: 60000, amount: CONFIG.amounts.large_order, desc: 'FOK large YES buy @ 60¢', order_type: 'FOK' },

            // Fill-and-Kill (FAK) - partial fills allowed, cancel remainder
            { account: 'trader2', outcome: 0, type: 'BuyShares', price: 40000, amount: CONFIG.amounts.large_order, desc: 'FAK large NO buy @ 40¢', order_type: 'FAK' },

            // Good-Till-Date with various expiries
            { account: 'market_maker', outcome: 1, type: 'SellShares', price: 70000, amount: CONFIG.amounts.medium_order, desc: 'GTD YES sell @ 70¢', order_type: 'GTD' },
            { account: 'market_maker', outcome: 0, type: 'SellShares', price: 50000, amount: CONFIG.amounts.medium_order, desc: 'GTD NO sell @ 50¢', order_type: 'GTD' },

            // Large whale order
            { account: 'trader1', outcome: 1, type: 'BuyShares', price: 57000, amount: CONFIG.amounts.whale_order, desc: 'Whale YES buy @ 57¢', order_type: 'GTC' },
        ];

        for (const order of advancedOrders) {
            const intent = {
                intent_id: `advanced_${Date.now()}_${Math.random()}`,
                user: CONFIG.accounts[order.account],
                market_id: this.marketId,
                intent_type: order.type,
                outcome: order.outcome,
                amount: order.amount,
                max_price: order.type === 'BuyShares' ? order.price : null,
                min_price: order.type === 'SellShares' ? order.price : null,
                deadline: (Date.now() + 24 * 60 * 60 * 1000) * 1000000,
                order_type: order.order_type
            };

            try {
                await this.submitIntent(intent, order.account, order.desc);
                await this.sleep(2500);
            } catch (error) {
                this.log(`⚠️  ${order.desc} failed: ${error.message}`);
            }
        }
    }

    async showOrderbookSummary() {
        this.log('\n📊 Checking Orderbook State...');

        try {
            const healthResponse = await fetch('http://localhost:8080/health');
            if (healthResponse.ok) {
                this.log('✅ Orderbook service is online');

                // Try to fetch orderbook data
                for (let outcome = 0; outcome <= 1; outcome++) {
                    const outcomeName = outcome === 1 ? 'YES' : 'NO';
                    try {
                        const response = await fetch(`http://localhost:8080/orderbook/${this.marketId}/${outcome}`);
                        if (response.ok) {
                            const orderbook = await response.json();
                            this.log(`📋 ${outcomeName} orderbook: ${orderbook.bids?.length || 0} bids, ${orderbook.asks?.length || 0} asks`);
                        }
                    } catch (e) {
                        this.log(`⚠️  Could not fetch ${outcomeName} orderbook`);
                    }
                }
            } else {
                this.log('⚠️  Orderbook service offline - orders processed via NEAR only');
            }
        } catch (error) {
            this.log('⚠️  Orderbook service not accessible - orders processed via NEAR only');
        }
    }

    async run() {
        this.log('🚀 Starting Intent-Focused Test');
        this.log(`📊 Market ID: ${this.marketId}`);

        try {
            await this.initializeNear();

            // Run comprehensive test suite with realistic patterns
            await this.testComplementaryOrders();
            await this.testMultipleBuyOrders();
            await this.testMultipleSellOrders();
            await this.testMarketOrders();
            await this.testRealisticMarketMaking();
            await this.testRetailTradingPatterns();
            await this.testAdvancedOrderTypes();

            // Show final state
            await this.showOrderbookSummary();

            this.log('\n🎉 INTENT TEST SUITE COMPLETE');
            this.log('📋 All intents submitted via NEAR chain');
            this.log('🔄 Processing handled by solver integration');

        } catch (error) {
            this.log(`❌ Test suite failed: ${error.message}`);
            process.exit(1);
        }
    }
}

// Run the test
if (require.main === module) {
    const tester = new IntentTester();
    tester.run().catch(console.error);
}

module.exports = IntentTester;