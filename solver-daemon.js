#!/usr/bin/env node

/**
 * Solver Daemon for NEAR Intent-based Prediction Marketplace
 * Monitors for verified intents and executes optimal solutions via orderbook
 */

const { execSync } = require('child_process');
const axios = require('axios');
const fs = require('fs');

const CONFIG = {
    NEAR_NETWORK: 'testnet',
    VERIFIER_CONTRACT: 'verifier.ashpk20.testnet',
    SOLVER_CONTRACT: 'solver.ashpk20.testnet',
    ACCOUNT_ID: 'ashpk20.testnet',
    ORDERBOOK_URL: 'http://localhost:8080',
    POLLING_INTERVAL: 10000, // 10 seconds (increased to avoid rate limits)
    SETTLEMENT_INTERVAL: 5000, // 5 seconds
    MAX_GAS: '300000000000000',
    // Retry configuration
    MAX_RETRY_ATTEMPTS: 5,
    INITIAL_RETRY_DELAY: 1000, // 1 second
    MAX_RETRY_DELAY: 60000, // 60 seconds
    INTENT_EXPIRY_TIME: 3600000, // 1 hour in milliseconds
};

class SolverDaemon {
    constructor() {
        this.isRunning = false;
        this.pendingIntents = new Map();
        this.failedIntents = new Map(); // Track intents that failed due to orderbook being offline
        this.pendingTrades = [];
        this.orderbookOnline = false;

        // Retry mechanism tracking
        this.intentRetryData = new Map(); // Map<intentId, {attempts, nextRetryTime, firstAttemptTime}>
        this.processingIntents = new Set(); // Track intents currently being processed

        // Point NEAR CLI to QuickNode (same as test script) to keep endpoints consistent.
        // Allow override via NEAR_RPC_URL if you want to switch quickly.
        const QUICKNODE = process.env.NEAR_RPC_URL || 'https://billowing-ancient-meadow.near-testnet.quiknode.pro/02fe7cae1f78374077f55e172eba1f849e8570f4/';
        process.env.NEAR_CLI_TESTNET_RPC_SERVER_URL = QUICKNODE;
    }

    async start() {
        console.log('🤖 Starting NEAR Intent Solver Daemon');
        console.log('=====================================');
        console.log(`Network: ${CONFIG.NEAR_NETWORK}`);
        console.log(`Verifier: ${CONFIG.VERIFIER_CONTRACT}`);
        console.log(`Solver: ${CONFIG.SOLVER_CONTRACT}`);
        console.log(`Account: ${CONFIG.ACCOUNT_ID}`);
        console.log('');

        this.isRunning = true;

        // Start monitoring loops
        this.startIntentMonitoring();
        this.startOrderbookHealthCheck();
        this.startSettlementLoop();
        this.startRetryLoop();

        console.log('✅ Solver daemon started successfully');
        console.log('⏳ Monitoring for intents...');
    }

    async startIntentMonitoring() {
        setInterval(async () => {
            if (!this.isRunning) return;
            
            try {
                await this.checkForNewIntents();
            } catch (error) {
                console.error('❌ Intent monitoring error:', error.message);
            }
        }, CONFIG.POLLING_INTERVAL);
    }

    async startOrderbookHealthCheck() {
        setInterval(async () => {
            if (!this.isRunning) return;
            
            try {
                const response = await axios.get(`${CONFIG.ORDERBOOK_URL}/health`);
                if (!this.orderbookOnline && response.status === 200) {
                    console.log('✅ Orderbook service is online');
                    this.orderbookOnline = true;
                    
                    // Retry failed intents now that orderbook is online
                    await this.retryFailedIntents();
                } else if (this.orderbookOnline && response.status !== 200) {
                    console.log('❌ Orderbook service is offline');
                    this.orderbookOnline = false;
                }
            } catch (error) {
                if (this.orderbookOnline) {
                    console.log('❌ Orderbook service is offline');
                    this.orderbookOnline = false;
                }
            }
        }, 10000); // Check every 10 seconds
    }

    async startSettlementLoop() {
        setInterval(async () => {
            if (!this.isRunning || this.pendingTrades.length === 0) return;
            
            try {
                await this.settlePendingTrades();
            } catch (error) {
                console.error('❌ Settlement error:', error.message);
            }
        }, CONFIG.SETTLEMENT_INTERVAL);
    }

    async startRetryLoop() {
        setInterval(async () => {
            if (!this.isRunning) return;

            try {
                await this.processRetryQueue();
                await this.cleanupExpiredIntents();
            } catch (error) {
                console.error('❌ Retry loop error:', error.message);
            }
        }, 5000); // Check every 5 seconds
    }

    calculateRetryDelay(attempts) {
        // Exponential backoff with jitter: delay = min(max_delay, initial_delay * 2^attempts) + random(0, 1000)
        const exponentialDelay = CONFIG.INITIAL_RETRY_DELAY * Math.pow(2, attempts);
        const cappedDelay = Math.min(exponentialDelay, CONFIG.MAX_RETRY_DELAY);
        const jitter = Math.random() * 1000; // Add up to 1 second of jitter
        return cappedDelay + jitter;
    }

    shouldRetryIntent(intentId) {
        const retryData = this.intentRetryData.get(intentId);
        if (!retryData) return true; // First attempt

        // Check if max attempts exceeded
        if (retryData.attempts >= CONFIG.MAX_RETRY_ATTEMPTS) {
            return false;
        }

        // Check if enough time has passed since last attempt
        return Date.now() >= retryData.nextRetryTime;
    }

    markIntentForRetry(intentId, error = null) {
        const now = Date.now();
        const retryData = this.intentRetryData.get(intentId) || {
            attempts: 0,
            firstAttemptTime: now
        };

        retryData.attempts += 1;
        retryData.nextRetryTime = now + this.calculateRetryDelay(retryData.attempts);
        retryData.lastError = error;

        this.intentRetryData.set(intentId, retryData);

        const nextRetryIn = Math.round((retryData.nextRetryTime - now) / 1000);
        console.log(`🔄 Intent ${intentId} scheduled for retry ${retryData.attempts}/${CONFIG.MAX_RETRY_ATTEMPTS} in ${nextRetryIn}s`);

        if (error) {
            console.log(`   Last error: ${error}`);
        }
    }

    async processRetryQueue() {
        const now = Date.now();
        const retryableIntents = [];

        // Find intents ready for retry
        for (const [intentId, retryData] of this.intentRetryData.entries()) {
            if (retryData.attempts < CONFIG.MAX_RETRY_ATTEMPTS && now >= retryData.nextRetryTime) {
                // Check if intent is still pending in the contract
                if (this.pendingIntents.has(intentId) && !this.processingIntents.has(intentId)) {
                    retryableIntents.push(intentId);
                }
            }
        }

        if (retryableIntents.length > 0) {
            console.log(`🔄 Processing ${retryableIntents.length} intents ready for retry...`);
        }

        // Process retryable intents
        for (const intentId of retryableIntents) {
            const intent = this.pendingIntents.get(intentId);
            if (intent) {
                console.log(`🔄 Retrying intent: ${intentId}`);
                await this.processIntent(intent);
            }
        }
    }

    async cleanupExpiredIntents() {
        const now = Date.now();
        const expiredIntents = [];

        // Find expired intents
        for (const [intentId, retryData] of this.intentRetryData.entries()) {
            const ageMs = now - retryData.firstAttemptTime;
            const maxAttemptsReached = retryData.attempts >= CONFIG.MAX_RETRY_ATTEMPTS;
            const expired = ageMs > CONFIG.INTENT_EXPIRY_TIME;

            if (maxAttemptsReached || expired) {
                expiredIntents.push(intentId);
            }
        }

        // Cleanup expired intents
        for (const intentId of expiredIntents) {
            const retryData = this.intentRetryData.get(intentId);
            const reason = retryData.attempts >= CONFIG.MAX_RETRY_ATTEMPTS
                ? `max retries (${retryData.attempts}) reached`
                : 'expired';

            console.log(`🗑️  Cleaning up intent ${intentId}: ${reason}`);

            // Remove from all tracking maps
            this.pendingIntents.delete(intentId);
            this.intentRetryData.delete(intentId);
            this.failedIntents.delete(intentId);
            this.processingIntents.delete(intentId);

            // Optionally notify the solver contract of permanent failure
            try {
                const executionResult = {
                    intent_id: intentId,
                    success: false,
                    output_amount: null,
                    fee_amount: "0",
                    execution_details: `Failed after ${retryData.attempts} attempts: ${retryData.lastError || 'Unknown error'}`
                };

                execSync(
                    `near call ${CONFIG.SOLVER_CONTRACT} complete_intent '{"intent_id": "${intentId}", "result": ${JSON.stringify(executionResult)}}' --accountId ${CONFIG.ACCOUNT_ID}`,
                    { encoding: 'utf8' }
                );
                console.log(`✅ Notified solver contract of permanent failure for ${intentId}`);
            } catch (notifyError) {
                console.log(`⚠️ Failed to notify solver contract of failure for ${intentId}:`, notifyError.message);
            }
        }
    }

    async checkForNewIntents() {
        try {
            // Query solver contract for pending intents (async solver pattern)
            const result = execSync(
                `NEAR_CLI_OUTPUT=json near view ${CONFIG.SOLVER_CONTRACT} get_pending_for_daemon '{}'`,
                { encoding: 'utf8' }
            );

            // Extract JSON result from NEAR CLI output
            // The response can be either single-line "[]" or multi-line JSON
            const lines = result.trim().split('\n');
            let jsonStr = '';
            
            // Strategy 1: Look for any line that looks like a JSON array (single line)
            for (const line of lines) {
                const trimmed = line.trim();
                // Check if line looks like JSON array (starts with [ and ends with ])
                if (trimmed.startsWith('[') && trimmed.endsWith(']')) {
                    jsonStr = trimmed;
                    console.log(`✅ Found single-line JSON array: "${trimmed}"`);
                    break;
                }
                // Also check if it's a valid JSON string that might be an array
                if (trimmed.startsWith('[')) {
                    try {
                        const testParse = JSON.parse(trimmed);
                        if (Array.isArray(testParse)) {
                            jsonStr = trimmed;
                            console.log(`✅ Found valid JSON array: "${trimmed}"`);
                            break;
                        }
                    } catch (e) {
                        // Continue looking
                    }
                }
            }
            
            // Strategy 2: If not found, look for multi-line JSON starting with '[' 
            if (!jsonStr) {
                console.log('🔍 Single-line JSON not found, trying multi-line...');
                let jsonStartLine = -1;
                for (let i = 0; i < lines.length; i++) {
                    const trimmed = lines[i].trim();
                    if (trimmed === '[' || trimmed.startsWith('[')) {
                        jsonStartLine = i;
                        console.log(`🔍 Found potential JSON start at line ${i}: "${trimmed}"`);
                        break;
                    }
                }
                
                if (jsonStartLine !== -1) {
                    // Reconstruct multi-line JSON
                    let jsonLines = [];
                    let bracketCount = 0;
                    
                    for (let i = jsonStartLine; i < lines.length; i++) {
                        const line = lines[i];
                        jsonLines.push(line);
                        
                        // Count brackets to know when we've finished the array
                        for (const char of line) {
                            if (char === '[') bracketCount++;
                            if (char === ']') bracketCount--;
                        }
                        
                        if (bracketCount === 0 && i > jsonStartLine) {
                            break;
                        }
                    }
                    
                    jsonStr = jsonLines.join('\n').trim();
                    console.log(`✅ Reconstructed multi-line JSON: "${jsonStr}"`);
                }
            }
            
            // Strategy 3: If still not found, try to find anything that looks like JSON in the output
            if (!jsonStr) {
                console.log('🔍 Multi-line JSON not found, scanning for any JSON-like content...');
                for (const line of lines) {
                    const trimmed = line.trim();
                    if (trimmed.length > 0 && (trimmed.startsWith('[') || trimmed.startsWith('{'))) {
                        try {
                            const testParse = JSON.parse(trimmed);
                            jsonStr = trimmed;
                            console.log(`✅ Found parseable JSON: "${trimmed}"`);
                            break;
                        } catch (e) {
                            console.log(`❌ Failed to parse potential JSON: "${trimmed}" - ${e.message}`);
                        }
                    }
                }
            }
            
            if (!jsonStr) {
                console.log('⚠️ No JSON array found in NEAR CLI response after all strategies, retrying later...');
                console.log('📋 Raw response:');
                console.log(result);
                console.log('📋 Individual lines:');
                lines.forEach((line, i) => console.log(`  Line ${i}: "${line}"`));
                return;
            }
            
            console.log('🔍 Raw NEAR CLI output:');
            console.log(result);
            console.log('🔍 Extracted JSON string:');
            console.log(jsonStr);
            
            let intents;
            try {
                // First try direct JSON parsing
                let parsed;
                try {
                    parsed = JSON.parse(jsonStr);
                } catch (directParseError) {
                    // If direct parsing fails, try to convert JavaScript object notation to proper JSON
                    console.log('🔄 Direct JSON parse failed, trying JS object notation conversion...');
                    const properJsonStr = jsonStr
                        .replace(/(\w+):/g, '"$1":')  // Add quotes around property names
                        .replace(/'/g, '"')           // Replace single quotes with double quotes
                        .replace(/null/g, 'null');    // Keep null as is
                    
                    console.log('🔍 Converted JSON string:');
                    console.log(properJsonStr);
                    
                    parsed = JSON.parse(properJsonStr);
                }
                
                // Ensure we have an array to iterate over
                if (Array.isArray(parsed)) {
                    intents = parsed;
                } else if (parsed === null || parsed === undefined) {
                    intents = [];
                } else {
                    console.log('⚠️ Response is not an array, wrapping in array. Response:', JSON.stringify(parsed, null, 2));
                    intents = [parsed];
                }
                
                console.log(`✅ Successfully parsed ${intents.length} intents`);
                
            } catch (parseError) {
                console.log('⚠️ Failed to parse JSON response:', parseError.message);
                console.log('📋 Failed JSON string:', jsonStr);
                console.log('⚠️ Retrying later...');
                return;
            }
            
            // intents is now an array of intent IDs, need to fetch full intent objects
            // Get all verified intents and filter by the pending IDs
            let allVerifiedIntents = [];
            try {
                const verifiedResult = execSync(
                    `NEAR_CLI_OUTPUT=json near view ${CONFIG.VERIFIER_CONTRACT} get_verified_intents '{}'`,
                    { encoding: 'utf8' }
                );
                
                // Parse verified intents - handle both single-line and multi-line JS object notation
                const verifiedLines = verifiedResult.trim().split('\n');
                let verifiedJsonStr = '';
                
                // Strategy 1: Look for single-line JSON array
                for (const line of verifiedLines) {
                    const trimmed = line.trim();
                    if (trimmed.startsWith('[') && trimmed.endsWith(']')) {
                        verifiedJsonStr = trimmed;
                        break;
                    }
                }
                
                // Strategy 2: If not found, reconstruct multi-line array
                if (!verifiedJsonStr) {
                    let arrayStartLine = -1;
                    for (let i = 0; i < verifiedLines.length; i++) {
                        const trimmed = verifiedLines[i].trim();
                        if (trimmed === '[' || trimmed.startsWith('[')) {
                            arrayStartLine = i;
                            break;
                        }
                    }
                    
                    if (arrayStartLine !== -1) {
                        // Find the end of the array
                        let bracketCount = 0;
                        let arrayLines = [];
                        
                        for (let i = arrayStartLine; i < verifiedLines.length; i++) {
                            const line = verifiedLines[i];
                            arrayLines.push(line);
                            
                            for (const char of line) {
                                if (char === '[') bracketCount++;
                                if (char === ']') bracketCount--;
                            }
                            
                            if (bracketCount === 0 && i > arrayStartLine) {
                                break;
                            }
                        }
                        
                        verifiedJsonStr = arrayLines.join('\n').trim();
                    }
                }
                
                if (verifiedJsonStr) {
                    try {
                        // First try direct JSON parsing
                        allVerifiedIntents = JSON.parse(verifiedJsonStr);
                    } catch (directError) {
                        // Convert JavaScript object notation to JSON
                        const properJsonStr = verifiedJsonStr
                            .replace(/(\w+):/g, '"$1":')  // Add quotes around property names
                            .replace(/'/g, '"')           // Replace single quotes with double quotes
                            .replace(/null/g, 'null');    // Keep null as is
                        
                        allVerifiedIntents = JSON.parse(properJsonStr);
                    }
                }
            } catch (fetchError) {
                console.log('⚠️ Failed to fetch verified intents:', fetchError.message);
                return;
            }
            
            // Collect intents that need processing for parallel execution
            const intentsToProcess = [];
            const retryIntentsToProcess = [];

            for (const intentId of intents) {
                // Validate intent ID
                if (!intentId || typeof intentId !== 'string') {
                    console.log('⚠️ Invalid intent ID received:', JSON.stringify(intentId, null, 2));
                    continue;
                }

                // Find the full intent object in verified intents
                const intent = allVerifiedIntents.find(i => i.intent_id === intentId);
                if (!intent) {
                    console.log(`⚠️ Intent ${intentId} not found in verified intents, skipping`);
                    continue;
                }

                // For now, bypass solver contract's processed check since it marks intents as processed
                // before daemon can handle them. The daemon should be the actual processor.
                // TODO: Implement proper async coordination between solver contract and daemon

                // Check if we should process this intent (new or ready for retry)
                const isNewIntent = !this.pendingIntents.has(intent.intent_id);
                const isReadyForRetry = this.shouldRetryIntent(intent.intent_id);
                const isCurrentlyProcessing = this.processingIntents.has(intent.intent_id);

                if (isNewIntent) {
                    console.log(`🎯 New intent for daemon processing: ${intent.intent_id}`);
                    console.log(`   Market: ${intent.market_id}`);
                    console.log(`   Type: ${intent.intent_type}`);
                    console.log(`   Amount: ${intent.amount}`);

                    this.pendingIntents.set(intent.intent_id, intent);
                    intentsToProcess.push(intent);
                } else if (isReadyForRetry && !isCurrentlyProcessing) {
                    console.log(`🔄 Intent ${intent.intent_id} ready for retry`);
                    retryIntentsToProcess.push(intent);
                } else if (isCurrentlyProcessing) {
                    console.log(`⏳ Intent ${intent.intent_id} currently being processed, skipping`);
                } else {
                    const retryData = this.intentRetryData.get(intent.intent_id);
                    if (retryData) {
                        const nextRetryIn = Math.round((retryData.nextRetryTime - Date.now()) / 1000);
                        console.log(`⏭️  Intent ${intent.intent_id} already processed, next retry in ${nextRetryIn}s`);
                    } else {
                        console.log(`⏭️  Intent ${intent.intent_id} already being processed by daemon, skipping`);
                    }
                }
            }

            // Process all new intents and retries in parallel
            const allIntentsToProcess = [...intentsToProcess, ...retryIntentsToProcess];
            if (allIntentsToProcess.length > 0) {
                console.log(`🚀 Processing ${allIntentsToProcess.length} intents in parallel across markets: ${[...new Set(allIntentsToProcess.map(i => i.market_id))].join(', ')}`);

                // Process intents concurrently
                const results = await Promise.allSettled(
                    allIntentsToProcess.map(intent => this.processIntent(intent))
                );

                // Log results
                results.forEach((result, index) => {
                    const intent = allIntentsToProcess[index];
                    if (result.status === 'fulfilled') {
                        console.log(`✅ Intent ${intent.intent_id} processed successfully`);
                    } else {
                        console.log(`❌ Intent ${intent.intent_id} failed: ${result.reason}`);
                    }
                });
            }
        } catch (error) {
            // Handle various NEAR CLI errors gracefully
            if (error.message.includes("doesn't exist")) {
                console.log('⚠️ Method doesn\'t exist, skipping...');
                return;
            }
            if (error.message.includes("Rate limits exceeded") || error.message.includes("TooManyRequestsError")) {
                console.log('⚠️ Rate limit exceeded, retrying later...');
                return;
            }
            if (error.message.includes("Unexpected end of JSON input")) {
                console.log('⚠️ Invalid JSON response, retrying later...');
                return;
            }
            
            // Only throw for unexpected errors
            console.error('❌ Unexpected error in intent monitoring:', error.message);
        }
    }

    async processIntent(intent) {
        const intentId = intent.intent_id;
        console.log(`⚙️  Processing intent ${intentId}...`);

        // Mark as currently processing to prevent concurrent processing
        this.processingIntents.add(intentId);

        try {
            if (!this.orderbookOnline) {
                console.log('❌ Cannot process intent: orderbook offline');
                this.failedIntents.set(intentId, intent);
                this.markIntentForRetry(intentId, 'Orderbook offline');
                return;
            }
            // Convert intent to orderbook order
            const order = this.convertIntentToOrder(intent);
            
            console.log('📤 Submitting order to orderbook:', JSON.stringify(order, null, 2));
            
            // Submit to orderbook
            const response = await axios.post(`${CONFIG.ORDERBOOK_URL}/solver/orders`, order);

            if (response.status === 200) {
                const trades = response.data.trades || [];
                console.log(`✅ Intent processed: ${trades.length} trades generated`);

                // Success - remove from all tracking
                this.pendingIntents.delete(intentId);
                this.intentRetryData.delete(intentId);
                this.failedIntents.delete(intentId);
                console.log(`📤 Intent ${intentId} submitted to orderbook, removed from pending`);

                // Notify solver contract that daemon has completed processing
                try {
                    const executionResult = {
                        intent_id: intentId,
                        success: true,
                        output_amount: trades.length > 0 ? trades[0].amount : null,
                        fee_amount: "0",
                        execution_details: `Processed ${trades.length} trades`
                    };

                    execSync(
                        `near call ${CONFIG.SOLVER_CONTRACT} complete_intent '{"intent_id": "${intentId}", "result": ${JSON.stringify(executionResult)}}' --accountId ${CONFIG.ACCOUNT_ID}`,
                        { encoding: 'utf8' }
                    );
                    console.log(`✅ Notified solver contract that ${intentId} is completed`);
                } catch (notifyError) {
                    console.log(`⚠️ Failed to notify solver contract:`, notifyError.message);
                }

                // Queue trades for settlement
                this.pendingTrades.push(...trades.map(trade => ({
                    ...trade,
                    intent_id: intentId,
                    timestamp: Date.now()
                })));
            } else {
                console.log(`❌ Failed to process intent: ${response.status}`);
                this.markIntentForRetry(intentId, `HTTP ${response.status}`);
            }
        } catch (error) {
            console.error(`❌ Error processing intent ${intentId}:`, error.message);
            this.markIntentForRetry(intentId, error.message);
        } finally {
            // Always remove from processing set
            this.processingIntents.delete(intentId);
        }
    }

    async retryFailedIntents() {
        if (this.failedIntents.size === 0) return;

        console.log(`🔄 Retrying ${this.failedIntents.size} failed intents now that orderbook is online...`);

        const failedIntentsArray = Array.from(this.failedIntents.values());
        this.failedIntents.clear(); // Clear the failed intents map

        for (const intent of failedIntentsArray) {
            const intentId = intent.intent_id;
            console.log(`🔄 Retrying intent: ${intentId}`);

            // Reset retry data for orderbook-offline failures when orderbook comes back online
            if (this.intentRetryData.has(intentId)) {
                const retryData = this.intentRetryData.get(intentId);
                if (retryData.lastError && retryData.lastError.includes('offline')) {
                    console.log(`🔄 Resetting retry count for ${intentId} due to orderbook recovery`);
                    retryData.attempts = 0;
                    retryData.nextRetryTime = Date.now();
                    this.intentRetryData.set(intentId, retryData);
                }
            }

            await this.processIntent(intent);
        }
    }

    // Utility method to manually clear stuck intents (for debugging/recovery)
    clearStuckIntent(intentId) {
        console.log(`🗑️  Manually clearing stuck intent: ${intentId}`);
        this.pendingIntents.delete(intentId);
        this.intentRetryData.delete(intentId);
        this.failedIntents.delete(intentId);
        this.processingIntents.delete(intentId);
        console.log(`✅ Cleared ${intentId} from all tracking maps`);
    }

    // Utility method to get current status of all intents
    getIntentStatus() {
        return {
            pending: Array.from(this.pendingIntents.keys()),
            retryData: Object.fromEntries(this.intentRetryData),
            failed: Array.from(this.failedIntents.keys()),
            processing: Array.from(this.processingIntents)
        };
    }

    // Get the latest market ID for synchronized processing
    getLatestMarketId() {
        try {
            // Try to read the latest market tracking file
            const data = fs.readFileSync('latest_market.json', 'utf8');
            const latestInfo = JSON.parse(data);
            if (latestInfo.latest_market_id) {
                return latestInfo.latest_market_id;
            }
        } catch (error) {
            // Fallback: Use most recent market from market_conditions.json
            try {
                const data = fs.readFileSync('market_conditions.json', 'utf8');
                const markets = JSON.parse(data);
                const marketKeys = Object.keys(markets).filter(k => k.includes('_ashpk20.testnet'));
                if (marketKeys.length > 0) {
                    return marketKeys.sort().pop(); // Get the last (highest) market ID
                }
            } catch (fallbackError) {
                console.error('Failed to read market conditions:', fallbackError.message);
            }
        }

        return 'market_1'; // Final fallback
    }

    getMarketConditionId(marketId) {
        try {
            const data = fs.readFileSync('market_conditions.json', 'utf8');
            const markets = JSON.parse(data);

            if (markets[marketId]) {
                return markets[marketId];
            } else {
                console.warn(`⚠️ No condition ID found for market ${marketId}`);
                return `fallback_condition_${marketId}`;
            }
        } catch (error) {
            console.error('Failed to read market conditions:', error.message);
            return `fallback_condition_${marketId}`;
        }
    }

    convertIntentToOrder(intent) {
        // Contracts now send prices in correct format (100000 = $1.00)
        let orderPrice = intent.max_price || intent.min_price || 0;

        // Validate price is within valid range
        if (orderPrice > 100000) {
            console.warn(`⚠️ Price ${orderPrice} exceeds 100000 (100%), setting to 0 for market order`);
            orderPrice = 0;
        }
        if (orderPrice < 0) {
            console.warn(`⚠️ Price ${orderPrice} is negative, setting to 0`);
            orderPrice = 0;
        }

        // Ensure order_type is valid (default to GTC for limit orders, Market for market orders)
        let orderType = intent.order_type || 'Market';
        const validOrderTypes = ['Market', 'GTC', 'FOK', 'GTD', 'FAK'];
        if (!validOrderTypes.includes(orderType)) {
            console.warn(`⚠️ Invalid order type ${orderType}, defaulting to GTC`);
            orderType = 'GTC';
        }

        return {
            order_id: `order_${intent.intent_id}`,
            intent_id: intent.intent_id,
            user: intent.user,
            market_id: intent.market_id,
            condition_id: this.getMarketConditionId(intent.market_id), // Use proper condition_id lookup
            outcome: intent.outcome,
            side: intent.intent_type === 'BuyShares' ? 'Buy' : 'Sell',
            order_type: orderType,
            price: orderPrice, // Now in new format (100000 = $1.00)
            amount: intent.amount,  // Keep as string for u128 compatibility
            filled_amount: "0",
            status: "Pending",
            created_at: Math.floor(Date.now() / 1000),
            expires_at: intent.deadline ? Math.floor(intent.deadline / 1000000) : Math.floor(Date.now() / 1000) + 3600
        };
    }

    async settlePendingTrades() {
        if (this.pendingTrades.length === 0) return;

        console.log(`🔧 Settling ${this.pendingTrades.length} pending trades...`);

        const tradesForSettlement = this.pendingTrades.splice(0, Math.min(5, this.pendingTrades.length));
        
        for (const trade of tradesForSettlement) {
            try {
                await this.settleTrade(trade);
            } catch (error) {
                console.error(`❌ Failed to settle trade ${trade.trade_id}:`, error.message);
                // Re-queue failed trade for retry
                this.pendingTrades.push({
                    ...trade,
                    retry_count: (trade.retry_count || 0) + 1,
                    timestamp: Date.now()
                });
            }
        }
    }

    async settleTrade(trade) {
        // Trade settlement is handled by the orderbook service via CTF contract
        // The solver daemon just needs to track that the trade is completed
        // since the orderbook already calls update_order_fill on both orders
        
        try {
            console.log(`✅ Trade settled by orderbook: ${trade.trade_id}`);
            
            // Remove from pending intents if all trades for this intent are settled
            if (trade.intent_id) {
                const remainingTrades = this.pendingTrades.filter(t => t.intent_id === trade.intent_id);
                if (remainingTrades.length === 0) {
                    this.pendingIntents.delete(trade.intent_id);
                    console.log(`✅ Intent fully executed: ${trade.intent_id}`);
                }
            }
        } catch (error) {
            console.error(`❌ Failed to process trade completion ${trade.trade_id}:`, error.message);
            return false;
        }
    }

    async stop() {
        console.log('🛑 Stopping solver daemon...');
        this.isRunning = false;
        console.log('✅ Solver daemon stopped');
    }

    // Utility method to check solver registration
    async checkSolverRegistration() {
        try {
            const result = execSync(
                `near view ${CONFIG.VERIFIER_CONTRACT} is_solver_registered '{"solver": "${CONFIG.SOLVER_CONTRACT}"}'`,
                { encoding: 'utf8' }
            );

            // Extract the boolean result from NEAR CLI output
            // Look for 'true' or 'false' in the output
            let isRegistered = false;
            const lines = result.trim().split('\n');
            
            for (const line of lines) {
                const trimmedLine = line.trim();
                if (trimmedLine === 'true') {
                    isRegistered = true;
                    break;
                } else if (trimmedLine === 'false') {
                    isRegistered = false;
                    break;
                }
            }
            
            if (isRegistered) {
                console.log('✅ Solver is properly registered');
            } else {
                console.log('❌ Solver is not registered. Please register first.');
                process.exit(1);
            }
        } catch (error) {
            console.error('❌ Failed to check solver registration:', error.message);
            process.exit(1);
        }
    }
}

// Handle graceful shutdown
process.on('SIGINT', async () => {
    console.log('\n🛑 Received SIGINT, shutting down gracefully...');
    if (global.solverDaemon) {
        await global.solverDaemon.stop();
    }
    process.exit(0);
});

process.on('SIGTERM', async () => {
    console.log('\n🛑 Received SIGTERM, shutting down gracefully...');
    if (global.solverDaemon) {
        await global.solverDaemon.stop();
    }
    process.exit(0);
});

// Main execution
async function main() {
    const daemon = new SolverDaemon();
    global.solverDaemon = daemon;
    
    // Check prerequisites
    console.log('🔍 Checking solver registration...');
    await daemon.checkSolverRegistration();
    
    // Start daemon
    await daemon.start();
    
    // Keep the process alive
    process.stdin.resume();
}

if (require.main === module) {
    main().catch(error => {
        console.error('❌ Fatal error:', error);
        process.exit(1);
    });
}

module.exports = { SolverDaemon, CONFIG };