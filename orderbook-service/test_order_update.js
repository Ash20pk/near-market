#!/usr/bin/env node

// Test script to debug order update logic and mapping
// This simulates the order flow to identify where the issue is occurring

const fs = require('fs');

// Simulate the data structures and flow
class OrderUpdateTest {
    constructor() {
        // Simulate the order_id_mapping (UUID -> solver_order_id)
        this.orderIdMapping = new Map();

        // Sample solver order that comes in
        this.sampleSolverOrder = {
            order_id: "order_yes_1758161696634_0.7739648341991108_1758161708275",
            intent_id: "intent_123",
            user: "user.testnet",
            market_id: "market_001",
            condition_id: "condition_001",
            outcome: 1,
            side: "Buy",
            order_type: "Limit",
            price: 5000,
            amount: "1000000",
            filled_amount: "0",
            status: "Pending",
            created_at: Date.now(),
            expires_at: 0
        };
    }

    // Step 1: Simulate order processing (what happens in process_solver_order)
    simulateOrderProcessing() {
        console.log("=== STEP 1: Processing Solver Order ===");
        console.log("Incoming solver order:", JSON.stringify(this.sampleSolverOrder, null, 2));

        // Generate UUID for orderbook (this is what we use internally)
        const orderbookOrderId = this.generateUUID();
        console.log(`Generated orderbook UUID: ${orderbookOrderId}`);

        // Store mapping: orderbook UUID -> solver order ID
        this.orderIdMapping.set(orderbookOrderId, this.sampleSolverOrder.order_id);
        console.log(`Stored mapping: ${orderbookOrderId} -> ${this.sampleSolverOrder.order_id}`);

        console.log("Order mapping contents:", Array.from(this.orderIdMapping.entries()));

        return {
            orderbookOrderId,
            solverOrderId: this.sampleSolverOrder.order_id
        };
    }

    // Step 2: Simulate trade generation (what happens when orders match)
    simulateTradeGeneration(makerOrderId, takerOrderId) {
        console.log("\n=== STEP 2: Generating Trade ===");

        // Simulate a trade being created
        const trade = {
            trade_id: this.generateUUID(),
            maker_order_id: makerOrderId,  // This is the orderbook UUID!
            taker_order_id: takerOrderId,  // This is the orderbook UUID!
            market_id: "market_001",
            condition_id: "condition_001",
            outcome: 1,
            price: 5000,
            size: 500000,
            maker_account: "maker.testnet",
            taker_account: "taker.testnet",
            executed_at: new Date()
        };

        console.log("Generated trade:", JSON.stringify(trade, null, 2));
        return trade;
    }

    // Step 3: Simulate settlement (what happens in settle_trade_via_solver)
    simulateSettlement(trade) {
        console.log("\n=== STEP 3: Settling Trade ===");
        console.log(`Looking up solver IDs for maker: ${trade.maker_order_id}, taker: ${trade.taker_order_id}`);
        console.log(`Current order mapping has ${this.orderIdMapping.size} entries`);

        // Look up solver order IDs from orderbook UUIDs
        const makerSolverId = this.orderIdMapping.get(trade.maker_order_id);
        const takerSolverId = this.orderIdMapping.get(trade.taker_order_id);

        if (!makerSolverId) {
            console.error(`❌ No solver ID found for maker order ${trade.maker_order_id}`);
            return false;
        }

        if (!takerSolverId) {
            console.error(`❌ No solver ID found for taker order ${trade.taker_order_id}`);
            return false;
        }

        console.log(`✅ Found solver IDs - maker: ${makerSolverId}, taker: ${takerSolverId}`);

        // Simulate the contract calls
        this.simulateContractCall("maker", makerSolverId, trade.size);
        this.simulateContractCall("taker", takerSolverId, trade.size);

        return true;
    }

    // Step 4: Simulate the actual contract call
    simulateContractCall(orderType, solverOrderId, filledAmount) {
        console.log(`\n--- Contract Call for ${orderType} ---`);

        const args = {
            order_id: solverOrderId,
            filled_amount: filledAmount.toString()
        };

        console.log(`Calling update_order_fill with args:`, JSON.stringify(args, null, 2));

        // This is where the actual error occurs - let's examine the order ID format
        console.log(`Order ID being sent: "${args.order_id}"`);
        console.log(`Order ID length: ${args.order_id.length}`);
        console.log(`Order ID format analysis:`);
        console.log(`  - Contains underscores: ${args.order_id.includes('_')}`);
        console.log(`  - Contains dots: ${args.order_id.includes('.')}`);
        console.log(`  - Contains numbers: ${/\d/.test(args.order_id)}`);

        // Check if this matches expected format
        const expectedPattern = /^order_(yes|no)_\d+_[\d.]+_\d+$/;
        console.log(`  - Matches expected pattern: ${expectedPattern.test(args.order_id)}`);

        return args;
    }

    // Test with multiple orders to see mapping behavior
    testMultipleOrders() {
        console.log("\n=== TESTING MULTIPLE ORDERS ===");

        // Create another solver order
        const solverOrder2 = {
            ...this.sampleSolverOrder,
            order_id: "order_no_1758161696635_0.1234567890123456_1758161708276",
            side: "Sell"
        };

        // Process both orders
        const order1 = this.simulateOrderProcessing();

        // Reset for second order
        this.sampleSolverOrder = solverOrder2;
        const order2 = this.simulateOrderProcessing();

        // Generate trade between them
        const trade = this.simulateTradeGeneration(order1.orderbookOrderId, order2.orderbookOrderId);

        // Try to settle
        return this.simulateSettlement(trade);
    }

    // Test what happens if mapping is missing
    testMissingMapping() {
        console.log("\n=== TESTING MISSING MAPPING ===");

        // Clear mapping to simulate the bug
        this.orderIdMapping.clear();

        const fakeUUID1 = this.generateUUID();
        const fakeUUID2 = this.generateUUID();

        const trade = this.simulateTradeGeneration(fakeUUID1, fakeUUID2);
        return this.simulateSettlement(trade);
    }

    generateUUID() {
        return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, function(c) {
            const r = Math.random() * 16 | 0;
            const v = c == 'x' ? r : (r & 0x3 | 0x8);
            return v.toString(16);
        });
    }

    // Main test runner
    runAllTests() {
        console.log("🔍 DEBUGGING ORDER UPDATE LOGIC\n");

        try {
            console.log("TEST 1: Single Order Flow");
            const result1 = this.simulateOrderProcessing();
            const trade1 = this.simulateTradeGeneration(result1.orderbookOrderId, result1.orderbookOrderId);
            this.simulateSettlement(trade1);

            console.log("\n" + "=".repeat(60));

            console.log("TEST 2: Multiple Orders");
            this.testMultipleOrders();

            console.log("\n" + "=".repeat(60));

            console.log("TEST 3: Missing Mapping (Bug Simulation)");
            this.testMissingMapping();

        } catch (error) {
            console.error("❌ Test failed:", error.message);
        }
    }
}

// Run the tests
const tester = new OrderUpdateTest();
tester.runAllTests();

console.log("\n🔧 POTENTIAL ISSUES TO CHECK:");
console.log("1. Are orderbook UUIDs being properly mapped to solver order IDs?");
console.log("2. Are the solver order IDs being retrieved correctly during settlement?");
console.log("3. Is the order ID format exactly what the contract expects?");
console.log("4. Are there any timing issues with the mapping storage/retrieval?");
console.log("5. Could the order have been removed from the contract before we try to update it?");

console.log("\n💡 DEBUGGING STEPS:");
console.log("1. Check the actual logs from the orderbook service");
console.log("2. Verify the order exists in the solver contract before trying to update");
console.log("3. Check if the order ID format matches exactly");
console.log("4. Consider if there are race conditions in order processing");