#!/bin/bash

# Test runner script for the orderbook service
# Runs all performance, concurrency, and integration tests

set -e

echo "🚀 Starting Orderbook Service Test Suite"
echo "========================================"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Function to run a test with timing
run_test() {
    local test_name=$1
    local test_command=$2
    
    echo -e "\n${BLUE}📊 Running $test_name...${NC}"
    echo "----------------------------------------"
    
    start_time=$(date +%s)
    
    if eval "$test_command"; then
        end_time=$(date +%s)
        duration=$((end_time - start_time))
        echo -e "${GREEN}✅ $test_name passed (${duration}s)${NC}"
        return 0
    else
        end_time=$(date +%s)
        duration=$((end_time - start_time))
        echo -e "${RED}❌ $test_name failed (${duration}s)${NC}"
        return 1
    fi
}

# Function to print test summary
print_summary() {
    local passed=$1
    local total=$2
    local failed=$((total - passed))
    
    echo -e "\n${BLUE}📋 Test Summary${NC}"
    echo "==============="
    echo -e "Total tests: $total"
    echo -e "${GREEN}Passed: $passed${NC}"
    
    if [ $failed -gt 0 ]; then
        echo -e "${RED}Failed: $failed${NC}"
    else
        echo -e "${GREEN}Failed: $failed${NC}"
    fi
    
    local success_rate=$((passed * 100 / total))
    echo -e "Success rate: $success_rate%"
}

# Performance benchmark function
run_performance_benchmark() {
    echo -e "\n${YELLOW}⚡ Performance Benchmarks${NC}"
    echo "========================="
    
    echo "Expected Performance Targets:"
    echo "• Order submission latency: <10ms average, <50ms P99"
    echo "• Throughput: >1,000 orders/second"
    echo "• Settlement: >100 trades/second"
    echo "• Concurrency: 10,000+ concurrent orders"
    echo ""
}

# Initialize test counters
total_tests=0
passed_tests=0

# Print banner
echo -e "${YELLOW}"
cat << 'EOF'
   ____          _           _                   _    
  / __ \        | |         | |                 | |   
 | |  | |_ __ __| | ___ _ __| |__   ___   ___ | | __
 | |  | | '__/ _` |/ _ \ '__| '_ \ / _ \ / _ \| |/ /
 | |__| | | | (_| |  __/ |  | |_) | (_) | (_) |   < 
  \____/|_|  \__,_|\___|_|  |_.__/ \___/ \___/|_|\_\
                                                    
     Performance & Concurrency Test Suite          
EOF
echo -e "${NC}"

run_performance_benchmark

# Check if we're in the right directory
if [ ! -f "Cargo.toml" ]; then
    echo -e "${RED}❌ Error: Cargo.toml not found. Please run from the orderbook-service directory.${NC}"
    exit 1
fi

# Check Rust installation
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}❌ Error: Cargo not found. Please install Rust.${NC}"
    exit 1
fi

echo -e "${BLUE}🔧 Building project...${NC}"
if ! cargo build --tests; then
    echo -e "${RED}❌ Build failed${NC}"
    exit 1
fi

echo -e "${GREEN}✅ Build successful${NC}"

# Run Performance Tests
echo -e "\n${YELLOW}🏁 PERFORMANCE TESTS${NC}"
echo "===================="

((total_tests++))
if run_test "Order Submission Latency" "cargo test test_order_submission_latency --test performance_tests -- --nocapture"; then
    ((passed_tests++))
fi

((total_tests++))
if run_test "Throughput Under Load" "cargo test test_throughput_under_load --test performance_tests -- --nocapture"; then
    ((passed_tests++))
fi

((total_tests++))
if run_test "Order Matching Accuracy" "cargo test test_order_matching_accuracy --test performance_tests -- --nocapture"; then
    ((passed_tests++))
fi

((total_tests++))
if run_test "Market Order Execution" "cargo test test_market_order_execution --test performance_tests -- --nocapture"; then
    ((passed_tests++))
fi

((total_tests++))
if run_test "Limit Order Behavior" "cargo test test_limit_order_behavior --test performance_tests -- --nocapture"; then
    ((passed_tests++))
fi

((total_tests++))
if run_test "Order Cancellation Under Load" "cargo test test_order_cancellation_under_load --test performance_tests -- --nocapture"; then
    ((passed_tests++))
fi

# Run Concurrency Tests
echo -e "\n${YELLOW}⚡ CONCURRENCY TESTS${NC}"
echo "==================="

((total_tests++))
if run_test "Simultaneous Matching" "cargo test test_simultaneous_matching_same_price --test concurrency_tests -- --nocapture"; then
    ((passed_tests++))
fi

((total_tests++))
if run_test "Order Cancellation Race Conditions" "cargo test test_order_cancellation_race_conditions --test concurrency_tests -- --nocapture"; then
    ((passed_tests++))
fi

((total_tests++))
if run_test "High-Frequency Small Orders" "cargo test test_high_frequency_small_orders --test concurrency_tests -- --nocapture"; then
    ((passed_tests++))
fi

((total_tests++))
if run_test "Market Order Liquidity Exhaustion" "cargo test test_market_order_liquidity_exhaustion --test concurrency_tests -- --nocapture"; then
    ((passed_tests++))
fi

((total_tests++))
if run_test "Cross-Market Isolation" "cargo test test_cross_market_isolation --test concurrency_tests -- --nocapture"; then
    ((passed_tests++))
fi

((total_tests++))
if run_test "Price-Time Priority" "cargo test test_price_time_priority --test concurrency_tests -- --nocapture"; then
    ((passed_tests++))
fi

((total_tests++))
if run_test "Order Expiration Accuracy" "cargo test test_order_expiration_accuracy --test concurrency_tests -- --nocapture"; then
    ((passed_tests++))
fi

# Run Settlement Tests
echo -e "\n${YELLOW}🔧 SETTLEMENT TESTS${NC}"
echo "==================="

((total_tests++))
if run_test "Settlement Throughput" "cargo test test_settlement_throughput --test settlement_tests -- --nocapture"; then
    ((passed_tests++))
fi

((total_tests++))
if run_test "Batch Settlement Efficiency" "cargo test test_batch_settlement_efficiency --test settlement_tests -- --nocapture"; then
    ((passed_tests++))
fi

((total_tests++))
if run_test "Settlement Failure Handling" "cargo test test_settlement_failure_handling --test settlement_tests -- --nocapture"; then
    ((passed_tests++))
fi

((total_tests++))
if run_test "Settlement Order Integrity" "cargo test test_settlement_order_integrity --test settlement_tests -- --nocapture"; then
    ((passed_tests++))
fi

((total_tests++))
if run_test "Settlement Gas Optimization" "cargo test test_settlement_gas_optimization --test settlement_tests -- --nocapture"; then
    ((passed_tests++))
fi

# Run Integration Tests
echo -e "\n${YELLOW}🔗 INTEGRATION TESTS${NC}"
echo "===================="

((total_tests++))
if run_test "End-to-End Order Flow" "cargo test test_end_to_end_order_flow --test integration_tests -- --nocapture"; then
    ((passed_tests++))
fi

((total_tests++))
if run_test "Market Order Immediate Execution" "cargo test test_market_order_immediate_execution --test integration_tests -- --nocapture"; then
    ((passed_tests++))
fi

((total_tests++))
if run_test "High Volume Stress Test" "cargo test test_high_volume_stress --test integration_tests -- --nocapture"; then
    ((passed_tests++))
fi

((total_tests++))
if run_test "Price Discovery Accuracy" "cargo test test_price_discovery_accuracy --test integration_tests -- --nocapture"; then
    ((passed_tests++))
fi

((total_tests++))
if run_test "Settlement Integration" "cargo test test_settlement_integration --test integration_tests -- --nocapture"; then
    ((passed_tests++))
fi

# Print final summary
print_summary $passed_tests $total_tests

# Exit with appropriate code
if [ $passed_tests -eq $total_tests ]; then
    echo -e "\n${GREEN}🎉 All tests passed! Orderbook is ready for production.${NC}"
    exit 0
else
    echo -e "\n${RED}⚠️  Some tests failed. Please review the failures above.${NC}"
    exit 1
fi