#!/bin/bash

# 📊 Status Monitor for Intent-Based Prediction Market
# Real-time monitoring of CTF + Intent System
# Shows: Contract health, Market activity, Bridge status, System metrics

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

echo -e "${PURPLE}📊 Intent-Based Prediction Market Status Monitor${NC}"
echo -e "${PURPLE}===============================================${NC}"
echo -e "${BLUE}🔍 Real-time CTF + Intent System Monitoring${NC}"
echo ""

# Get master account
get_master_account() {
    if [ -f "deployment-summary.json" ]; then
        MASTER_ACCOUNT=$(cat deployment-summary.json | grep '"masterAccount"' | cut -d'"' -f4)
        echo -e "${GREEN}✅ Found deployment: $MASTER_ACCOUNT${NC}"
    else
        echo -e "${BLUE}🔐 Enter your NEAR testnet account ID:${NC}"
        read -p "Account ID (e.g., yourname.testnet): " MASTER_ACCOUNT
        
        if [[ ! "$MASTER_ACCOUNT" == *.testnet ]]; then
            echo -e "${RED}❌ Please use a .testnet account${NC}"
            exit 1
        fi
    fi
}

# Check contract deployment status
check_contract_status() {
    echo -e "${BLUE}📦 Contract Deployment Status${NC}"
    echo "================================"
    
    local contracts=("ctf" "resolver" "verifier" "solver")
    
    for contract in "${contracts[@]}"; do
        local contract_account="$contract.$MASTER_ACCOUNT"
        
        if near state $contract_account &> /dev/null; then
            local balance=$(near state $contract_account | grep "amount:" | awk '{print $2}' | sed 's/[",]//g')
            echo -e "${GREEN}✅ $contract: $contract_account ($balance NEAR)${NC}"
        else
            echo -e "${RED}❌ $contract: Not deployed${NC}"
        fi
    done
    echo ""
}

# Check CTF system health
check_ctf_health() {
    echo -e "${BLUE}🎯 CTF System Health${NC}"
    echo "===================="
    
    if near state ctf.$MASTER_ACCOUNT &> /dev/null; then
        echo "📊 CTF Statistics:"
        
        # Check collateral token registration
        if near view ctf.$MASTER_ACCOUNT is_collateral_token_registered \
           '{"token": "3e2210e1184b45b64c8a434c0a7e7b23cc04ea7eb7a6c3c32520d03d4afcb8af"}' 2>/dev/null | grep -q "true"; then
            echo -e "${GREEN}   ✅ USDC collateral registered${NC}"
        else
            echo -e "${RED}   ❌ USDC collateral not registered${NC}"
        fi
        
        echo -e "${GREEN}   ✅ CTF system operational${NC}"
    else
        echo -e "${RED}❌ CTF contract not deployed${NC}"
    fi
    echo ""
}

# Check cross-chain bridge status
check_bridge_status() {
    echo -e "${BLUE}🌉 Cross-Chain Bridge Status${NC}"
    echo "============================="
    
    # Bridge statistics
    echo "Bridge Statistics:"
    near view verifier.$MASTER_ACCOUNT get_bridge_stats '{}' 2>/dev/null || echo -e "${YELLOW}⚠️ Stats unavailable${NC}"
    
    # Security status
    echo "Bridge Security:"
    local paused=$(near view verifier.$MASTER_ACCOUNT is_bridge_paused '{}' 2>/dev/null || echo "unknown")
    if [ "$paused" = "false" ]; then
        echo -e "${GREEN}✅ Bridge operational${NC}"
    elif [ "$paused" = "true" ]; then
        echo -e "${RED}🚨 Bridge paused (emergency mode)${NC}"
    else
        echo -e "${YELLOW}⚠️ Status unknown${NC}"
    fi
}

# Check monitoring system
check_monitoring() {
    echo -e "${BLUE}📊 Monitoring System Status${NC}"
    echo "============================"
    
    # Check if monitor is deployed
    if near state monitor.$MASTER_ACCOUNT &> /dev/null; then
        # Get monitoring stats
        near view monitor.$MASTER_ACCOUNT get_monitoring_stats '{}' 2>/dev/null || echo -e "${YELLOW}⚠️ Monitoring unavailable${NC}"
        
        # Check retry queue
        echo "Active transactions being monitored:"
        near view monitor.$MASTER_ACCOUNT get_active_transactions_count '{}' 2>/dev/null || echo -e "${YELLOW}⚠️ Count unavailable${NC}"
    else
        echo -e "${YELLOW}⚠️ Monitor contract not deployed${NC}"
    fi
}

# Performance metrics
check_performance() {
    echo -e "${BLUE}⚡ Performance Metrics${NC}"
    echo "======================"
    
    # Get account balance
    local balance=$(near state $MASTER_ACCOUNT | grep "amount:" | awk '{print $2}' | sed 's/[",]//g')
    echo -e "Master Account Balance: ${GREEN}$balance NEAR${NC}"
    
    # Storage usage across all contracts
    echo "Total Storage Usage:"
    local total_storage=0
    local contracts=("verifier" "solver" "ctf" "resolver")
    
    for contract in "${contracts[@]}"; do
        if near state $contract.$MASTER_ACCOUNT &> /dev/null; then
            local storage=$(near state $contract.$MASTER_ACCOUNT | grep "storage_usage:" | awk '{print $2}')
            total_storage=$((total_storage + storage))
        fi
    done
    
    echo -e "${GREEN}${total_storage} bytes total${NC}"
}

# Health check summary
health_check_summary() {
    echo -e "${BLUE}🏥 Health Check Summary${NC}"
    echo "======================="
    
    local healthy=0
    local total=5
    local contracts=("verifier" "solver" "ctf" "resolver")
    
    for contract in "${contracts[@]}"; do
        if near state $contract.$MASTER_ACCOUNT &> /dev/null; then
            healthy=$((healthy + 1))
        fi
    done
    
    local health_percentage=$((healthy * 100 / total))
    
    if [ $health_percentage -eq 100 ]; then
        echo -e "${GREEN}🎉 All systems operational (${healthy}/${total})${NC}"
    elif [ $health_percentage -ge 80 ]; then
        echo -e "${YELLOW}⚠️ Mostly operational (${healthy}/${total})${NC}"
    else
        echo -e "${RED}🚨 System issues detected (${healthy}/${total})${NC}"
    fi
    
    # Recent activity check
    echo "Recent Activity Check:"
    echo -e "${BLUE}Check NEAR Explorer for recent transactions:${NC}"
    for contract in "${contracts[@]}"; do
        echo "https://explorer.testnet.near.org/accounts/$contract.$MASTER_ACCOUNT"
    done
}

# Generate status report
generate_status_report() {
    echo -e "${BLUE}📋 Generating Status Report${NC}"
    
    local report_file="status-report-$(date +%Y%m%d-%H%M%S).json"
    
    # Get basic stats
    local healthy_contracts=0
    local contracts=("verifier" "solver" "ctf" "resolver")
    
    for contract in "${contracts[@]}"; do
        if near state $contract.$MASTER_ACCOUNT &> /dev/null; then
            healthy_contracts=$((healthy_contracts + 1))
        fi
    done
    
    cat > $report_file << EOF
{
  "statusCheck": {
    "timestamp": "$(date -u +"%Y-%m-%dT%H:%M:%SZ")",
    "network": "testnet",
    "masterAccount": "$MASTER_ACCOUNT"
  },
  "contractsStatus": {
    "totalContracts": 5,
    "healthyContracts": $healthy_contracts,
    "healthPercentage": $((healthy_contracts * 100 / 5))
  },
  "contracts": {
    "verifier": "$([ -n "$(near state verifier.$MASTER_ACCOUNT 2>/dev/null)" ] && echo "healthy" || echo "down")",
    "solver": "$([ -n "$(near state solver.$MASTER_ACCOUNT 2>/dev/null)" ] && echo "healthy" || echo "down")",
    "ctf": "$([ -n "$(near state ctf.$MASTER_ACCOUNT 2>/dev/null)" ] && echo "healthy" || echo "down")",
    "resolver": "$([ -n "$(near state resolver.$MASTER_ACCOUNT 2>/dev/null)" ] && echo "healthy" || echo "down")",
    "usdcContract": "3e2210e1184b45b64c8a434c0a7e7b23cc04ea7eb7a6c3c32520d03d4afcb8af"
  }
}
EOF
    
    echo -e "${GREEN}✅ Status report saved: $report_file${NC}"
}

# Check system integration status
check_integration_status() {
    echo -e "${BLUE}🔗 System Integration Status${NC}"
    echo "==========================="
    
    local all_healthy=true
    
    # Check CTF -> Resolver integration
    if near state ctf.$MASTER_ACCOUNT &> /dev/null && near state resolver.$MASTER_ACCOUNT &> /dev/null; then
        echo -e "${GREEN}   ✅ CTF ↔ Resolver integration${NC}"
    else
        echo -e "${RED}   ❌ CTF ↔ Resolver integration${NC}"
        all_healthy=false
    fi
    
    # Check Verifier -> Solver integration
    if near state verifier.$MASTER_ACCOUNT &> /dev/null && near state solver.$MASTER_ACCOUNT &> /dev/null; then
        echo -e "${GREEN}   ✅ Verifier ↔ Solver integration${NC}"
    else
        echo -e "${RED}   ❌ Verifier ↔ Solver integration${NC}"
        all_healthy=false
    fi
    
    # Check full system integration
    if $all_healthy; then
        echo -e "${GREEN}   ✅ Full system integration healthy${NC}"
    else
        echo -e "${YELLOW}   ⚠️ Some integrations need attention${NC}"
    fi
    
    echo ""
}

# Main status check
main() {
    echo "Checking deployment status..."
    echo ""
    
    get_master_account
    echo ""
    
    check_contract_status
    check_ctf_health
    check_integration_status
    
    echo ""
    echo -e "${GREEN}🎯 Status Check Complete!${NC}"
    echo -e "${BLUE}Use './test.sh' to run functional tests${NC}"
    echo -e "${BLUE}Use './configure.sh' to configure contracts${NC}"
}

# Handle script arguments
case "${1:-all}" in
    "all"|"")
        main
        ;;
    "quick")
        echo -e "${YELLOW}Running quick health check...${NC}"
        echo ""
        get_master_account
        check_contract_status
        check_integration_status
        echo -e "${GREEN}✅ Quick health check complete${NC}"
        ;;
    "contracts")
        get_master_account
        check_contract_status
        ;;
    "ctf")
        get_master_account
        check_ctf_health
        ;;
    *)
        echo "Usage: $0 [all|quick|contracts|ctf]"
        echo "  all       - Complete status check (default)"
        echo "  quick     - Quick health summary"
        echo "  contracts - Contract deployment status only"
        echo "  ctf       - CTF system health only"
        ;;
esac