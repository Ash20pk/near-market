#!/usr/bin/env node

/**
 * Check Outcome Token Balances
 * Verifies if traders received their YES/NO outcome tokens after trading
 */

const { connect, keyStores } = require('near-api-js');
const fs = require('fs');
const os = require('os');
const path = require('path');

const CONFIG = {
    network: 'testnet',
    contracts: {
        ctf: 'ctf.ashpk20.testnet',
        usdc: '3e2210e1184b45b64c8a434c0a7e7b23cc04ea7eb7a6c3c32520d03d4afcb8af'
    },
    accounts: {
        master: 'ashpk20.testnet',
        market_maker: 'market-maker.ashpk20.testnet',
        trader1: 'trader-joe-1.ashpk20.testnet',
        trader2: 'trader-joe-2.ashpk20.testnet'
    }
};

class OutcomeTokenChecker {
    constructor() {
        this.near = null;
        this.nearConfig = {
            networkId: CONFIG.network,
            nodeUrl: 'https://billowing-ancient-meadow.near-testnet.quiknode.pro/02fe7cae1f78374077f55e172eba1f849e8570f4/',
            keyStore: new keyStores.UnencryptedFileSystemKeyStore(path.join(os.homedir(), '.near-credentials')),
        };
    }

    log(message) {
        const timestamp = new Date().toISOString();
        console.log(`[${timestamp}] ${message}`);
    }

    async initialize() {
        this.log('🚀 Initializing NEAR connection...');
        this.near = await connect(this.nearConfig);
    }

    async nearView(contractId, method, args = {}) {
        const account = await this.near.account(CONFIG.accounts.master);
        try {
            return await account.viewFunction(contractId, method, args);
        } catch (error) {
            // Try with string conversion for AccountId parameters
            const stringifiedArgs = {};
            for (const [key, value] of Object.entries(args)) {
                stringifiedArgs[key] = value.toString();
            }
            return await account.viewFunction(contractId, method, stringifiedArgs);
        }
    }

    /**
     * Check all outcome tokens for a specific user
     */
    async checkUserOutcomeTokens(accountId) {
        this.log(`\n🔍 Checking outcome tokens for ${accountId}...`);
        
        try {
            // Get all positions for this user
            const positions = await this.nearView(CONFIG.contracts.ctf, 'get_user_positions', {
                user: accountId
            });

            if (positions.length === 0) {
                this.log(`   📊 No outcome tokens found`);
                return [];
            }

            this.log(`   📊 Found ${positions.length} position(s):`);
            
            for (const [positionId, balance] of positions) {
                const balanceNum = parseInt(balance);
                const balanceUSDC = balanceNum / 1000000; // Convert from 6 decimals
                
                // Try to decode the position to understand what it represents
                const outcomeType = this.decodePositionType(positionId);
                
                this.log(`      • Position: ${positionId}`);
                this.log(`        Type: ${outcomeType}`);
                this.log(`        Balance: ${balanceUSDC} tokens (${balance} raw)`);
            }

            return positions;
        } catch (error) {
            this.log(`   ❌ Error checking positions: ${error.message}`);
            return [];
        }
    }

    /**
     * Try to decode position type (YES/NO) from position ID
     */
    decodePositionType(positionId) {
        // Position IDs contain encoded information about the collection and outcomes
        // For binary markets: outcome 1 = YES, outcome 2 = NO
        if (positionId.includes(':1')) {
            return '🟢 YES tokens';
        } else if (positionId.includes(':2')) {
            return '🔴 NO tokens';
        } else {
            return '🔵 Unknown outcome';
        }
    }

    /**
     * Check specific outcome token balance
     */
    async checkSpecificOutcome(accountId, conditionId, outcome) {
        this.log(`\n🎯 Checking ${outcome === 1 ? 'YES' : 'NO'} tokens for ${accountId}...`);
        
        try {
            // Generate collection ID for the specific outcome
            // For binary outcomes: YES = [1], NO = [2] 
            const indexSet = outcome === 1 ? ["1"] : ["2"];
            
            const collectionId = await this.nearView(CONFIG.contracts.ctf, 'get_collection_id', {
                parent_collection_key: "",
                condition_id: conditionId,
                index_set: indexSet
            });

            // Get position ID for this collection
            const positionId = await this.nearView(CONFIG.contracts.ctf, 'get_position_id', {
                collateral_token: CONFIG.contracts.usdc,
                collection_id: collectionId
            });

            // Check balance
            const balance = await this.nearView(CONFIG.contracts.ctf, 'balance_of', {
                owner: accountId,
                position_id: positionId
            });

            const balanceNum = parseInt(balance);
            const balanceUSDC = balanceNum / 1000000;
            
            this.log(`   📊 ${outcome === 1 ? 'YES' : 'NO'} Token Balance: ${balanceUSDC} tokens`);
            this.log(`   🔗 Position ID: ${positionId}`);
            
            return { balance: balanceNum, positionId };
            
        } catch (error) {
            this.log(`   ❌ Error checking ${outcome === 1 ? 'YES' : 'NO'} tokens: ${error.message}`);
            return { balance: 0, positionId: null };
        }
    }

    /**
     * Check outcome tokens for all accounts
     */
    async checkAllAccounts() {
        this.log('\n🎯 OUTCOME TOKEN BALANCE REPORT');
        this.log('=================================');

        const accounts = [
            CONFIG.accounts.master,
            CONFIG.accounts.market_maker, 
            CONFIG.accounts.trader1,
            CONFIG.accounts.trader2
        ];

        let totalPositions = 0;
        
        for (const account of accounts) {
            const positions = await this.checkUserOutcomeTokens(account);
            totalPositions += positions.length;
            
            // Add small delay between checks
            await new Promise(resolve => setTimeout(resolve, 1000));
        }

        this.log(`\n📊 Summary: Found ${totalPositions} total outcome token positions across all accounts`);
    }

    /**
     * Check specific market outcome tokens
     */
    async checkMarketOutcomes(conditionId) {
        this.log(`\n🏛️  MARKET OUTCOME ANALYSIS`);
        this.log(`Condition ID: ${conditionId}`);
        this.log('=================================');

        const accounts = [
            CONFIG.accounts.market_maker,
            CONFIG.accounts.trader1, 
            CONFIG.accounts.trader2
        ];

        for (const account of accounts) {
            this.log(`\n👤 ${account}:`);
            
            // Check YES tokens (outcome 1)
            await this.checkSpecificOutcome(account, conditionId, 1);
            
            // Check NO tokens (outcome 0) 
            await this.checkSpecificOutcome(account, conditionId, 0);
            
            await new Promise(resolve => setTimeout(resolve, 500));
        }
    }

    /**
     * Main execution
     */
    async run() {
        try {
            await this.initialize();
            
            // Get condition ID from command line or use default
            const conditionId = process.argv[2];
            
            if (conditionId) {
                this.log(`🎯 Checking market with condition ID: ${conditionId}`);
                await this.checkMarketOutcomes(conditionId);
            } else {
                this.log(`🔍 Checking all outcome tokens across all accounts...`);
                await this.checkAllAccounts();
                
                this.log(`\n💡 To check a specific market, run:`);
                this.log(`   node check-outcome-tokens.js <condition_id>`);
            }
            
        } catch (error) {
            this.log(`❌ Error: ${error.message}`);
            process.exit(1);
        }
    }
}

// Usage examples in comments
if (require.main === module) {
    const checker = new OutcomeTokenChecker();
    checker.run();
}

/*
Usage Examples:

1. Check all outcome tokens for all accounts:
   node check-outcome-tokens.js

2. Check specific market by condition ID:
   node check-outcome-tokens.js 9befe926b52053fb966d45a6fca15146e66b6a342450cd07c51608e2a75a3710

3. Find condition ID from market_conditions.json:
   cat orderbook-service/market_conditions.json | grep condition_id
*/