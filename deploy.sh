#!/bin/bash

# 🚀 Deploy Script for Intent-Based Prediction Market
# Complete Polymarket CTF + NEAR Intent System Deployment
# Handles proper dependency order: CTF → Resolver → Verifier → Solver

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
DEPLOYMENT_COST="2"   # NEAR tokens for contract deployments (reduced for testnet)

# Contract deployment order (dependency-based)
CONTRACTS_ORDER=(
    "ctf"       # Foundation contract (no dependencies)
    "resolver"  # Depends on CTF for resolution
    "monitor"   # Cross-chain bridge monitoring (independent)
    "verifier"  # Depends on CTF + Resolver + Monitor
    "solver"    # Depends on CTF + Verifier + Monitor
)

echo -e "${PURPLE}🎯 Intent-Based Prediction Market Deployment${NC}"
echo -e "${PURPLE}=============================================${NC}"
echo -e "${BLUE}📦 System: Polymarket CTF + NEAR Intent Architecture${NC}"
echo -e "${BLUE}🔗 Deployment Order: ${CONTRACTS_ORDER[*]}${NC}"
echo ""

# Get master account from user
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

# Check prerequisites
check_prerequisites() {
    echo -e "${BLUE}📋 Checking deployment prerequisites...${NC}"
    
    # Check NEAR CLI
    if ! command -v near &> /dev/null; then
        echo -e "${RED}❌ NEAR CLI not found. Install with: npm install -g near-cli${NC}"
        exit 1
    fi
    
    # Check if logged in
    if ! near state $MASTER_ACCOUNT &> /dev/null; then
        echo -e "${RED}❌ Not logged in or account doesn't exist${NC}"
        echo "Please run: near login"
        exit 1
    fi
    
    # Check WASM files exist (workshop structure)
    if [ ! -d "res" ]; then
        echo -e "${RED}❌ res directory not found. Run ./build.sh first${NC}"
        exit 1
    fi
    
    local required_contracts=("ctf.wasm" "resolver.wasm" "monitor.wasm" "verifier.wasm" "solver.wasm")
    for contract in "${required_contracts[@]}"; do
        if [ ! -f "res/$contract" ]; then
            echo -e "${RED}❌ $contract not found in res/. Run ./build.sh first${NC}"
            exit 1
        fi
    done
    
    # Check account balance
    local balance=$(near state $MASTER_ACCOUNT | grep "amount:" | awk '{print $2}' | sed 's/[",]//g')
    echo -e "${GREEN}💰 Account balance: $balance NEAR${NC}"
    
    echo -e "${GREEN}✅ Prerequisites verified${NC}"
}

# Create subaccounts for each contract
create_subaccounts() {
    echo -e "${BLUE}🏗️ Creating contract subaccounts...${NC}"
    
    for contract in "${CONTRACTS_ORDER[@]}"; do
        local subaccount="$contract.$MASTER_ACCOUNT"
        
        echo "Creating: $subaccount"
        
        # Check if account already exists
        if near state $subaccount &> /dev/null; then
            echo -e "${YELLOW}⚠️ Account $subaccount already exists${NC}"
        else
            near create-account $subaccount --masterAccount $MASTER_ACCOUNT --initialBalance $DEPLOYMENT_COST
            echo -e "${GREEN}✅ Created: $subaccount${NC}"
        fi
    done
    
    echo -e "${GREEN}✅ Subaccounts ready${NC}"
}

# Deploy contracts in dependency order
deploy_contracts() {
    echo -e "${BLUE}📦 Deploying contracts in dependency order...${NC}"
    
    # 1. Deploy CTF (Foundation - no dependencies)
    echo -e "${YELLOW}[1/5] Deploying CTF Contract (Polymarket-style)...${NC}"
    near deploy ctf.$MASTER_ACCOUNT res/ctf.wasm --force
    near call ctf.$MASTER_ACCOUNT new "{\"owner\": \"$MASTER_ACCOUNT\"}" --accountId $MASTER_ACCOUNT
    echo -e "${GREEN}   ✅ CTF deployed: ctf.$MASTER_ACCOUNT${NC}"
    
    # Register USDC as collateral in CTF
    echo -e "${YELLOW}   Registering USDC as collateral...${NC}"
    near call ctf.$MASTER_ACCOUNT register_collateral_token \
        '{"token": "3e2210e1184b45b64c8a434c0a7e7b23cc04ea7eb7a6c3c32520d03d4afcb8af"}' \
        --accountId $MASTER_ACCOUNT \
        --gas 30000000000000
    
    # 2. Deploy Resolver (depends on CTF)
    echo -e "${YELLOW}[2/5] Deploying Resolver Contract...${NC}"
    near deploy resolver.$MASTER_ACCOUNT res/resolver.wasm --force
    near call resolver.$MASTER_ACCOUNT new "{\"owner_id\": \"$MASTER_ACCOUNT\", \"verifier_contract\": \"verifier.$MASTER_ACCOUNT\", \"ctf_contract\": \"ctf.$MASTER_ACCOUNT\", \"dispute_period\": 86400000000000, \"dispute_bond\": \"100000000000000000000000\"}" --accountId $MASTER_ACCOUNT
    echo -e "${GREEN}   ✅ Resolver deployed: resolver.$MASTER_ACCOUNT${NC}"
    
    # 3. Deploy Monitor (Cross-chain bridge monitoring)
    echo -e "${YELLOW}[3/5] Deploying Monitor Contract...${NC}"
    near deploy monitor.$MASTER_ACCOUNT res/monitor.wasm --force
    near call monitor.$MASTER_ACCOUNT new "{\"owner_id\": \"$MASTER_ACCOUNT\"}" --accountId $MASTER_ACCOUNT
    echo -e "${GREEN}   ✅ Monitor deployed: monitor.$MASTER_ACCOUNT${NC}"
    
    # 4. Deploy Verifier (depends on CTF + Resolver + Monitor)
    echo -e "${YELLOW}[4/5] Deploying Verifier Contract...${NC}"
    near deploy verifier.$MASTER_ACCOUNT res/verifier.wasm --force
    near call verifier.$MASTER_ACCOUNT new "{\"owner_id\": \"$MASTER_ACCOUNT\", \"ctf_contract\": \"ctf.$MASTER_ACCOUNT\", \"resolver_contract\": \"resolver.$MASTER_ACCOUNT\", \"min_bet_amount\": \"1000000\", \"max_bet_amount\": \"1000000000000\", \"platform_fee_bps\": 100}" --accountId $MASTER_ACCOUNT
    echo -e "${GREEN}   ✅ Verifier deployed: verifier.$MASTER_ACCOUNT${NC}"
    
    # 5. Deploy Solver (depends on CTF + Verifier + Monitor)
    echo -e "${YELLOW}[5/5] Deploying Solver Contract...${NC}"
    near deploy solver.$MASTER_ACCOUNT res/solver.wasm --force
    near call solver.$MASTER_ACCOUNT new "{\"owner_id\": \"$MASTER_ACCOUNT\", \"verifier_contract\": \"verifier.$MASTER_ACCOUNT\", \"ctf_contract\": \"ctf.$MASTER_ACCOUNT\", \"usdc_contract\": \"3e2210e1184b45b64c8a434c0a7e7b23cc04ea7eb7a6c3c32520d03d4afcb8af\", \"orderbook_authority\": \"$MASTER_ACCOUNT\", \"solver_fee_bps\": 50, \"min_order_size\": \"1000000\"}" --accountId $MASTER_ACCOUNT
    echo -e "${GREEN}   ✅ Solver deployed: solver.$MASTER_ACCOUNT${NC}"
    
    echo ""
    echo -e "${GREEN}🎉 All contracts deployed in correct dependency order!${NC}"
}

# Configure contract relationships
configure_contracts() {
    echo -e "${BLUE}⚙️ Configuring contract relationships...${NC}"
    
    # Register Solver with Verifier
    echo "Registering solver with verifier..."
    near call verifier.$MASTER_ACCOUNT register_solver \
        "{\"solver\": \"solver.$MASTER_ACCOUNT\"}" \
        --accountId $MASTER_ACCOUNT \
        --gas 30000000000000
    
    # # Configure cross-chain bridge (demo setup)
    # echo "Configuring cross-chain bridge..."
    # near call verifier.$MASTER_ACCOUNT configure_bridge \
    #     "{\"bridge_contract\": \"bridge-connector.testnet\", \"supported_chains\": [1, 137]}" \
    #     --accountId $MASTER_ACCOUNT \
    #     --gas 30000000000000
    
    # # Register Solver with Verifier (WORKSHOP REQUIREMENT)
    # echo "Registering solver with verifier..."
    # near call verifier.$MASTER_ACCOUNT register_solver \
    #     "{\"solver\": \"solver.$MASTER_ACCOUNT\"}" \
    #     --accountId $MASTER_ACCOUNT \
    #     --gas 30000000000000
    
    # # Set monitor contract in solver (if method exists)
    # echo "Linking monitor contract to solver..."
    # near call solver.$MASTER_ACCOUNT set_monitor_contract \
    #     "{\"monitor_contract\": \"monitor.$MASTER_ACCOUNT\"}" \
    #     --accountId $MASTER_ACCOUNT \
    #     --gas 30000000000000 || echo "⚠️ set_monitor_contract method not available"
    
    # # Enable cross-chain functionality in solver
    # echo "Enabling cross-chain functionality..."
    # near call solver.$MASTER_ACCOUNT toggle_cross_chain \
    #     "{\"enabled\": true}" \
    #     --accountId $MASTER_ACCOUNT \
    #     --gas 30000000000000
    
    echo -e "${GREEN}✅ Configuration complete${NC}"
}

# Verify deployments
verify_deployment() {
    echo -e "${BLUE}🔍 Verifying deployments...${NC}"
    
    for contract in "${CONTRACTS_ORDER[@]}"; do
        local contract_account="$contract.$MASTER_ACCOUNT"
        echo "Checking: $contract_account"
        
        if near state $contract_account &> /dev/null; then
            echo -e "${GREEN}✅ $contract_account is deployed${NC}"
        else
            echo -e "${RED}❌ $contract_account deployment failed${NC}"
        fi
    done
    
    echo ""
    echo -e "${GREEN}🎉 Deployment Complete!${NC}"
    echo -e "${BLUE}Contract Addresses:${NC}"
    echo "=================="
    for contract in "${CONTRACTS_ORDER[@]}"; do
        echo "$contract: $contract.$MASTER_ACCOUNT"
    done
}

# Create deployment summary
create_deployment_summary() {
    local summary_file="deployment-summary.json"
    
    echo -e "${BLUE}📋 Creating deployment summary...${NC}"
    
    cat > $summary_file << EOF
{
  "network": "$NETWORK",
  "deployedAt": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
  "masterAccount": "$MASTER_ACCOUNT",
  "note": "USDC addresses sourced from usdc-addresses.json (official Circle contracts)",
  "contracts": {
    "verifier": "verifier.$MASTER_ACCOUNT",
    "solver": "solver.$MASTER_ACCOUNT",
    "smartWallet": "smart-wallet.$MASTER_ACCOUNT",
    "monitor": "monitor.$MASTER_ACCOUNT",
    "ctf": "ctf.$MASTER_ACCOUNT",
    "resolver": "resolver.$MASTER_ACCOUNT",
    "usdcContract": "3e2210e1184b45b64c8a434c0a7e7b23cc04ea7eb7a6c3c32520d03d4afcb8af"
  },
  "configuration": {
    "minBetAmount": "1000000",
    "maxBetAmount": "1000000000000",
    "platformFeeBps": 100,
    "solverFeeBps": 50,
    "crossChainEnabled": true
  },
  "frontend": {
    "networkConfig": {
      "networkId": "testnet",
      "nodeUrl": "https://rpc.testnet.near.org",
      "walletUrl": "https://wallet.testnet.near.org",
      "helperUrl": "https://helper.testnet.near.org"
    }
  }
}
EOF
    
    echo -e "${GREEN}✅ Summary saved to: $summary_file${NC}"
}

# Main deployment function
main() {
    echo "Starting deployment process..."
    echo ""
    
    get_master_account
    check_prerequisites
    create_subaccounts
    deploy_contracts
    configure_contracts
    verify_deployment
    create_deployment_summary
    
    echo ""
    echo -e "${PURPLE}🎯 Deployment Complete!${NC}"
    echo -e "${GREEN}Your Cross-Chain Prediction Market is now live on NEAR testnet!${NC}"
    echo ""
    echo -e "${BLUE}Next steps:${NC}"
    echo "1. Run './test.sh' to test your deployment"
    echo "2. Check './status.sh' to monitor contract status"
    echo "3. Open the frontend demo in your browser"
    echo ""
    echo -e "${YELLOW}📋 Contract Explorer Links:${NC}"
    for contract in "${CONTRACTS_ORDER[@]}"; do
        echo "https://explorer.testnet.near.org/accounts/$contract.$MASTER_ACCOUNT"
    done
}

# Handle script arguments
case "${1:-deploy}" in
    "deploy")
        main
        ;;
    "verify")
        get_master_account
        verify_deployment
        ;;
    "configure")
        get_master_account
        configure_contracts
        ;;
    *)
        echo "Usage: $0 [deploy|verify|configure]"
        echo "  deploy    - Full deployment (default)"
        echo "  verify    - Verify existing deployment"
        echo "  configure - Configure contract relationships"
        ;;
esac