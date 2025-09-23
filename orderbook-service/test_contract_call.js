#!/usr/bin/env node

// Test script to directly test the solver contract update_order_fill function
// This will help us understand why the contract is returning "Order not found"

const { execSync } = require('child_process');

class ContractCallTest {
    constructor() {
        // Use the same order ID from your error message
        this.testOrderId = "order_yes_1758161696634_0.7739648341991108_1758161708275";
        this.contractId = "solver.ashpk20.testnet"; // Update this to your actual contract
        this.filledAmount = "1000000";
    }

    // Test 1: Check if the order exists in the contract
    async checkOrderExists() {
        console.log("=== TEST 1: Checking if order exists ===");

        try {
            const cmd = `near view ${this.contractId} get_order '{"order_id": "${this.testOrderId}"}'`;
            console.log(`Running: ${cmd}`);

            const result = execSync(cmd, { encoding: 'utf8' });
            console.log("✅ Order found:", result);
            return true;
        } catch (error) {
            console.log("❌ Order not found or error:", error.message);
            return false;
        }
    }

    // Test 2: List all orders to see what's actually in the contract
    async listAllOrders() {
        console.log("\n=== TEST 2: Listing all orders ===");

        try {
            const cmd = `near view ${this.contractId} get_orders '{"from_index": 0, "limit": 10}'`;
            console.log(`Running: ${cmd}`);

            const result = execSync(cmd, { encoding: 'utf8' });
            console.log("📋 All orders:", result);

            // Parse and analyze the orders
            const orders = JSON.parse(result);
            if (Array.isArray(orders)) {
                console.log(`\n📊 Found ${orders.length} orders:`);
                orders.forEach((order, index) => {
                    console.log(`  ${index + 1}. ID: "${order.order_id}" (length: ${order.order_id.length})`);
                    console.log(`     Status: ${order.status}`);
                    console.log(`     Filled: ${order.filled_amount}`);
                });
            }

            return orders;
        } catch (error) {
            console.log("❌ Failed to list orders:", error.message);
            return [];
        }
    }

    // Test 3: Test the update_order_fill call with dry run
    async testUpdateCall() {
        console.log("\n=== TEST 3: Testing update_order_fill call ===");

        const args = {
            order_id: this.testOrderId,
            filled_amount: this.filledAmount
        };

        console.log("📞 Call arguments:", JSON.stringify(args, null, 2));

        try {
            // First try a view call to see if there's a way to validate
            const cmd = `near call ${this.contractId} update_order_fill '${JSON.stringify(args)}' --accountId test.testnet --dry-run`;
            console.log(`Running: ${cmd}`);

            const result = execSync(cmd, { encoding: 'utf8' });
            console.log("✅ Dry run successful:", result);
            return true;
        } catch (error) {
            console.log("❌ Dry run failed:", error.message);

            // Extract the actual error from the output
            const errorOutput = error.message;
            if (errorOutput.includes("Order not found")) {
                console.log("🔍 Confirmed: Order not found error");
            } else if (errorOutput.includes("panicked")) {
                const panicMatch = errorOutput.match(/panicked at [^:]+:(\d+):(\d+):[^\\n]*(\\n[^"]*)/);
                if (panicMatch) {
                    console.log("🔍 Panic location:", panicMatch[0]);
                }
            }

            return false;
        }
    }

    // Test 4: Compare order ID formats
    analyzeOrderIdFormat() {
        console.log("\n=== TEST 4: Analyzing order ID format ===");

        const orderId = this.testOrderId;
        console.log(`Order ID: "${orderId}"`);
        console.log(`Length: ${orderId.length}`);
        console.log(`Bytes: ${Buffer.from(orderId).toString('hex')}`);

        // Check for any unusual characters
        const chars = orderId.split('');
        const charAnalysis = chars.map((char, index) => ({
            index,
            char,
            code: char.charCodeAt(0),
            isAlnum: /[a-zA-Z0-9]/.test(char)
        }));

        console.log("Character analysis:");
        charAnalysis.forEach(({ index, char, code, isAlnum }) => {
            if (!isAlnum && char !== '_' && char !== '.') {
                console.log(`  Unusual char at ${index}: '${char}' (code: ${code})`);
            }
        });

        // Test different variations
        console.log("\nTesting variations:");
        const variations = [
            orderId,                           // Original
            orderId.trim(),                    // Trimmed
            orderId.replace(/\s+/g, ''),      // No whitespace
            JSON.stringify(orderId).slice(1, -1) // JSON encoded/decoded
        ];

        variations.forEach((variation, index) => {
            console.log(`  ${index}: "${variation}" (length: ${variation.length})`);
            if (variation !== orderId) {
                console.log(`     Different from original: ${variation === orderId ? 'No' : 'Yes'}`);
            }
        });
    }

    // Test 5: Check the contract state and recent activity
    async checkContractState() {
        console.log("\n=== TEST 5: Checking contract state ===");

        try {
            // Check contract state
            const stateCmd = `near view ${this.contractId} get_state`;
            console.log(`Running: ${stateCmd}`);

            const state = execSync(stateCmd, { encoding: 'utf8' });
            console.log("📊 Contract state:", state);

        } catch (error) {
            console.log("❌ Failed to get contract state:", error.message);
        }

        try {
            // Check recent transactions
            console.log("\n🔍 Checking recent activity...");
            const activityCmd = `near view ${this.contractId} get_recent_activity '{"limit": 5}'`;

            const activity = execSync(activityCmd, { encoding: 'utf8' });
            console.log("📈 Recent activity:", activity);

        } catch (error) {
            console.log("❌ Failed to get recent activity:", error.message);
        }
    }

    // Main test runner
    async runAllTests() {
        console.log("🔍 TESTING SOLVER CONTRACT CALLS\n");
        console.log(`Target contract: ${this.contractId}`);
        console.log(`Test order ID: ${this.testOrderId}`);
        console.log(`Fill amount: ${this.filledAmount}\n`);

        // Run all tests
        const orderExists = await this.checkOrderExists();
        const allOrders = await this.listAllOrders();

        this.analyzeOrderIdFormat();

        if (!orderExists && allOrders.length > 0) {
            console.log("\n🤔 ANALYSIS: Order doesn't exist but contract has other orders");
            console.log("Possible causes:");
            console.log("1. Order was already fully filled and removed");
            console.log("2. Order ID format doesn't match exactly");
            console.log("3. Order expired and was cleaned up");
            console.log("4. Wrong contract or environment");
        }

        await this.testUpdateCall();
        await this.checkContractState();

        console.log("\n💡 RECOMMENDATIONS:");
        console.log("1. Verify the order still exists before trying to update it");
        console.log("2. Add a check in the orderbook service to handle missing orders gracefully");
        console.log("3. Consider implementing idempotent updates");
        console.log("4. Add better error handling for already-completed orders");
    }
}

// Check if near CLI is available
try {
    execSync('near --version', { encoding: 'utf8' });
    console.log("✅ NEAR CLI found");
} catch (error) {
    console.log("❌ NEAR CLI not found. Please install near-cli: npm install -g near-cli");
    process.exit(1);
}

// Run the tests
const tester = new ContractCallTest();
tester.runAllTests().catch(console.error);