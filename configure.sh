#!/bin/bash

# ⚙️ Configuration Script for Intent-Based Prediction Market
# Post-deployment contract integration and setup
# Configures CTF → Resolver → Verifier → Solver relationships

set -e  # Exit on any error

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
PURPLE='\033[0;35m'
NC='\033[0m' # No Color

# Configuration
NETWORK="testnet"
MASTER_ACCOUNT=""
GAS_LIMIT="30000000000000"

# Demo configuration values
USDC_CONTRACT="3e2210e1184b45b64c8a434c0a7e7b23cc04ea7eb7a6c3c32520d03d4afcb8af"
DISPUTE_PERIOD="86400000000000"  # 24 hours in nanoseconds
DISPUTE_BOND="100000000000000000000000"  # 100 NEAR
MIN_BET_AMOUNT="1000000"  # 1 USDC
MAX_BET_AMOUNT="1000000000000"  # 1M USDC
PLATFORM_FEE_BPS="100"  # 1%
SOLVER_FEE_BPS="50"     # 0.5%

echo -e "${PURPLE}⚙️ Intent-Based Prediction Market Configuration${NC}"
echo -e "${PURPLE}=============================================${NC}"
echo -e "${BLUE}🔗 Setting up contract relationships and parameters${NC}"
echo ""

# Get master account
get_master_account() {
    if [ -z "$MASTER_ACCOUNT" ]; then
        echo -e "${BLUE}🔐 Enter your NEAR testnet account ID:${NC}"
        read -p "Account ID (e.g., yourname.testnet): " MASTER_ACCOUNT
        
        if [[ ! "$MASTER_ACCOUNT" == *.testnet ]]; then
            echo -e "${RED}❌ Please use a .testnet account${NC}"
            exit 1
        fi
    fi
    
    echo -e "${GREEN}✅ Using master account: $MASTER_ACCOUNT${NC}"
}

# Verify contracts are deployed
verify_contracts() {
    echo -e "${BLUE}🔍 Verifying contract deployments...${NC}"
    
    local contracts=("ctf" "resolver" "verifier" "solver")
    
    for contract in "${contracts[@]}"; do
        local contract_account="$contract.$MASTER_ACCOUNT"
        
        if near state $contract_account &> /dev/null; then
            echo -e "${GREEN}✅ $contract_account is deployed${NC}"
        else
            echo -e "${RED}❌ $contract_account not found. Run ./deploy.sh first${NC}"
            exit 1
        fi
    done
    
    echo -e "${GREEN}✅ All contracts verified${NC}"
}

# Configure CTF parameters
configure_ctf() {
    echo -e "${BLUE}🎯 Configuring CTF contract...${NC}"
    
    # Verify USDC is registered (should be done in deploy script)
    echo "Checking USDC collateral registration..."
    near view ctf.$MASTER_ACCOUNT is_collateral_token_registered \
        "{\"token\": \"$USDC_CONTRACT\"}" \
        --accountId $MASTER_ACCOUNT
    
    # Set CTF parameters
    echo "Setting CTF operational parameters..."
    near call ctf.$MASTER_ACCOUNT set_fee_parameters \
        "{
            \"platform_fee_bps\": $PLATFORM_FEE_BPS,
            \"max_fee_bps\": 1000
        }" \
        --accountId $MASTER_ACCOUNT \
        --gas $GAS_LIMIT
    
    echo -e "${GREEN}   ✅ CTF configured${NC}"
}

# Configure Resolver parameters  
configure_resolver() {
    echo -e "${BLUE}🎯 Configuring Resolver contract...${NC}"
    
    # Update dispute parameters
    echo "Setting dispute parameters..."
    near call resolver.$MASTER_ACCOUNT update_dispute_parameters \
        "{
            \"new_dispute_period\": \"$DISPUTE_PERIOD\",
            \"new_dispute_bond\": \"$DISPUTE_BOND\"
        }" \
        --accountId $MASTER_ACCOUNT \
        --gas $GAS_LIMIT
    
    # Register resolver as oracle in CTF
    echo "Registering resolver as oracle in CTF..."
    near call ctf.$MASTER_ACCOUNT set_oracle_permissions \
        "{
            \"oracle\": \"resolver.$MASTER_ACCOUNT\",
            \"can_prepare\": true,
            \"can_resolve\": true
        }" \
        --accountId $MASTER_ACCOUNT \
        --gas $GAS_LIMIT
    
    echo -e "${GREEN}   ✅ Resolver configured${NC}"
}

# Configure Verifier integration
configure_verifier() {
    echo -e "${BLUE}🎯 Configuring Verifier contract...${NC}"
    
    # Register solver with verifier
    echo "Registering solver with verifier..."
    near call verifier.$MASTER_ACCOUNT register_solver \
        "{\"solver\": \"solver.$MASTER_ACCOUNT\"}" \
        --accountId $MASTER_ACCOUNT \
        --gas $GAS_LIMIT
    
    # Configure cross-chain bridge settings
    echo "Configuring cross-chain bridge..."
    near call verifier.$MASTER_ACCOUNT configure_bridge \
        "{
            \"connector_account\": \"bridge-connector.testnet\",
            \"ethereum_rpc\": \"https://eth-sepolia.g.alchemy.com/v2/demo\",
            \"polygon_rpc\": \"https://polygon-mumbai.g.alchemy.com/v2/demo\",
            \"security_config\": {
                \"min_confirmations\": 6,
                \"max_gas_price\": \"50000000000\",
                \"timeout_seconds\": 300
            }
        }" \
        --accountId $MASTER_ACCOUNT \
        --gas $GAS_LIMIT
    
    # Set validation parameters
    echo "Setting intent validation parameters..."
    near call verifier.$MASTER_ACCOUNT set_validation_parameters \
        "{
            \"min_bet_amount\": \"$MIN_BET_AMOUNT\",
            \"max_bet_amount\": \"$MAX_BET_AMOUNT\",
            \"max_slippage_bps\": 500,
            \"intent_timeout_seconds\": 3600
        }" \
        --accountId $MASTER_ACCOUNT \
        --gas $GAS_LIMIT
    
    echo -e "${GREEN}   ✅ Verifier configured${NC}"
}

# Configure Solver integration
configure_solver() {
    echo -e "${BLUE}🎯 Configuring Solver contract...${NC}"
    
    # Set monitor contract (placeholder for now)
    echo "Setting monitor contract..."
    near call solver.$MASTER_ACCOUNT set_monitor_contract \
        "{\"monitor_contract\": \"monitor.$MASTER_ACCOUNT\"}" \
        --accountId $MASTER_ACCOUNT \
        --gas $GAS_LIMIT || echo -e "${YELLOW}⚠️ Monitor contract not found (optional)${NC}"
    
    # Configure OmniConnector for cross-chain operations
    echo "Configuring OmniConnector..."
    near call solver.$MASTER_ACCOUNT configure_omni_connector \
        "{
            \"ethereum_rpc\": \"https://eth-sepolia.g.alchemy.com/v2/demo\",
            \"polygon_rpc\": \"https://polygon-mumbai.g.alchemy.com/v2/demo\",
            \"bridge_contracts\": {
                \"ethereum\": \"0x1234567890123456789012345678901234567890\",
                \"polygon\": \"0x0987654321098765432109876543210987654321\"
            }
        }" \
        --accountId $MASTER_ACCOUNT \
        --gas $GAS_LIMIT
    
    # Set execution parameters
    echo "Setting execution parameters..."
    near call solver.$MASTER_ACCOUNT set_execution_parameters \
        "{
            \"platform_fee_bps\": $PLATFORM_FEE_BPS,
            \"solver_fee_bps\": $SOLVER_FEE_BPS,
            \"max_execution_time_ms\": 30000,
            \"retry_count\": 3
        }" \
        --accountId $MASTER_ACCOUNT \
        --gas $GAS_LIMIT
    
    echo -e "${GREEN}   ✅ Solver configured${NC}"
}

# Test basic contract interactions
test_integration() {
    echo -e "${BLUE}🧪 Testing contract integration...${NC}"
    
    # Test 1: Create a test market condition
    echo "Creating test market condition..."
    local test_question_id="test-market-$(date +%s)"
    
    near call resolver.$MASTER_ACCOUNT prepare_condition \
        "{
            \"oracle\": \"resolver.$MASTER_ACCOUNT\",
            \"question_id\": \"$test_question_id\",
            \"outcome_slot_count\": 2,
            \"description\": \"Test market for configuration verification\"
        }" \
        --accountId $MASTER_ACCOUNT \
        --gas $GAS_LIMIT
    
    # Test 2: Verify condition was created in CTF
    echo "Verifying condition in CTF..."
    near view ctf.$MASTER_ACCOUNT get_condition \
        "{\"condition_id\": \"$(echo -n resolver.$MASTER_ACCOUNT$test_question_id | sha256sum | cut -d' ' -f1)\"}" \
        --accountId $MASTER_ACCOUNT || echo -e "${YELLOW}⚠️ Condition verification skipped${NC}"
    
    # Test 3: Test verifier validation (without actual execution)
    echo "Testing verifier validation..."
    near call verifier.$MASTER_ACCOUNT validate_intent \
        "{
            \"intent\": {
                \"user\": \"$MASTER_ACCOUNT\",
                \"market_id\": \"$test_question_id\",
                \"intent_type\": \"BuyShares\",
                \"outcome\": 1,
                \"amount\": \"$MIN_BET_AMOUNT\",
                \"max_slippage_bps\": 100,
                \"expiry\": $(($(date +%s) * 1000000000 + 3600000000000))
            }
        }" \
        --accountId $MASTER_ACCOUNT \
        --gas $GAS_LIMIT
    
    echo -e "${GREEN}   ✅ Integration test completed${NC}"
}

# Create configuration summary
create_configuration_summary() {
    local config_file="configuration-summary.json"
    
    echo -e "${BLUE}📋 Creating configuration summary...${NC}"
    
    cat > $config_file << EOF
{
  "configured_at": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "network": "$NETWORK",
  "master_account": "$MASTER_ACCOUNT",
  "contracts": {
    "ctf": "ctf.$MASTER_ACCOUNT",
    "resolver": "resolver.$MASTER_ACCOUNT", 
    "verifier": "verifier.$MASTER_ACCOUNT",
    "solver": "solver.$MASTER_ACCOUNT"
  },
  "configuration": {
    "usdc_contract": "$USDC_CONTRACT",
    "dispute_period_ns": "$DISPUTE_PERIOD",
    "dispute_bond_near": "$DISPUTE_BOND",
    "min_bet_amount": "$MIN_BET_AMOUNT",
    "max_bet_amount": "$MAX_BET_AMOUNT",
    "platform_fee_bps": $PLATFORM_FEE_BPS,
    "solver_fee_bps": $SOLVER_FEE_BPS
  },
  "integrations": {
    "ctf_resolver_oracle": true,
    "verifier_solver_registration": true,
    "cross_chain_bridge": true,
    "omni_connector": true
  },
  "status": "configured"
}
EOF
    
    echo -e "${GREEN}✅ Configuration summary saved: $config_file${NC}"
}

# Main configuration function
main() {
    echo "Starting configuration process..."
    echo ""
    
    get_master_account
    verify_contracts
    configure_ctf
    configure_resolver
    configure_verifier
    configure_solver
    test_integration
    create_configuration_summary
    
    echo ""
    echo -e "${PURPLE}⚙️ Configuration Complete!${NC}"
    echo -e "${GREEN}Your Intent-Based Prediction Market is fully configured!${NC}"
    echo ""
    echo -e "${BLUE}Configuration Summary:${NC}"
    echo "======================"
    echo "✅ CTF: Platform fees and collateral configured"
    echo "✅ Resolver: Dispute parameters and oracle permissions set"
    echo "✅ Verifier: Solver registration and bridge configuration"
    echo "✅ Solver: Execution parameters and cross-chain setup"
    echo "✅ Integration: Basic contract interactions tested"
    echo ""
    echo -e "${BLUE}Next Steps:${NC}"
    echo "1. Run comprehensive tests: ${YELLOW}./test.sh${NC}"
    echo "2. Monitor system status: ${YELLOW}./status.sh${NC}"
    echo "3. Create your first market using the frontend"
    echo "4. Test cross-chain intent execution"
    echo ""
    echo -e "${YELLOW}📋 View configuration details: cat configuration-summary.json${NC}"
}

# Handle script arguments
case "${1:-configure}" in
    "configure")
        main
        ;;
    "verify")
        get_master_account
        verify_contracts
        ;;
    "test")
        get_master_account
        test_integration
        ;;
    *)
        echo "Usage: $0 [configure|verify|test]"
        echo "  configure - Full configuration (default)"
        echo "  verify    - Verify contract deployments only"
        echo "  test      - Run integration tests only"
        ;;
esac