#!/bin/bash

# 🏗️ Build Script for Intent-Based Prediction Market
# Complete Polymarket-style CTF + NEAR Intent System
# Workshop-aligned deployment with real CTF integration

set -e  # Exit on any error

echo "🚀 Building Intent-Based Prediction Market System"
echo "=================================================="
echo "🎯 Polymarket CTF + NEAR Intent Architecture"
echo "📦 4 Contracts: CTF → Resolver → Verifier → Solver"
echo ""

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

# Check prerequisites
check_prerequisites() {
    echo -e "${BLUE}📋 Checking prerequisites...${NC}"
    
    if ! command -v rustc &> /dev/null; then
        echo -e "${RED}❌ Rust not found. Please install Rust first.${NC}"
        exit 1
    fi
    
    if ! command -v near &> /dev/null; then
        echo -e "${RED}❌ NEAR CLI not found. Please install with: npm install -g near-cli${NC}"
        exit 1
    fi
    
    # Check if wasm32 target is installed
    if ! rustc --print target-list | grep -q "wasm32-unknown-unknown"; then
        echo -e "${YELLOW}⚠️ Installing wasm32 target...${NC}"
        rustup target add wasm32-unknown-unknown
    fi
    
    # Check for cargo-near (recommended for optimized builds)
    if command -v cargo-near &> /dev/null; then
        echo -e "${GREEN}✅ cargo-near found (optimized builds enabled)${NC}"
    else
        echo -e "${YELLOW}⚠️ cargo-near not found. Install for optimized builds:${NC}"
        echo -e "${YELLOW}   cargo install cargo-near${NC}"
    fi
    
    echo -e "${GREEN}✅ Prerequisites checked${NC}"
}

# Clean previous builds
clean_build() {
    echo -e "${BLUE}🧹 Cleaning previous builds...${NC}"
    
    # Only clean res directory and individual contract targets
    if [ -d "res" ]; then
        rm -rf res
        echo "Removed res directory"
    fi
    
    # Clean individual contract targets
    if [ -d "contracts" ]; then
        for contract_dir in contracts/*/; do
            if [ -d "$contract_dir" ]; then
                contract_name=$(basename "$contract_dir")
                if [ -d "$contract_dir/target" ]; then
                    rm -rf "$contract_dir/target"
                    echo "Removed $contract_name/target directory"
                fi
            fi
        done
    fi
    
    echo -e "${GREEN}✅ Clean complete${NC}"
}

# Build contracts in dependency order
build_contracts() {
    echo -e "${BLUE}🔨 Building contracts in dependency order...${NC}"
    
    # Create res directory for WASM outputs
    mkdir -p res
    
    # Define build order based on dependencies
    # CTF (foundation) → Resolver → Monitor → Verifier → Solver
    contracts_order=("ctf" "resolver" "monitor" "verifier" "solver")
    
    echo -e "${YELLOW}📋 Build Order: ${contracts_order[*]}${NC}"
    echo ""
    
    for contract_name in "${contracts_order[@]}"; do
        contract_dir="contracts/$contract_name"
        
        if [ ! -d "$contract_dir" ]; then
            echo -e "${RED}❌ Contract directory not found: $contract_dir${NC}"
            exit 1
        fi
        
        echo -e "${BLUE}🔨 Building $contract_name...${NC}"
        
        # Build in the contract's directory
        cd "$contract_dir"
        
        # First check compilation
        if ! cargo check --target wasm32-unknown-unknown; then
            echo -e "${RED}❌ $contract_name compilation check failed${NC}"
            cd - > /dev/null
            exit 1
        fi
        
        # Try cargo-near first, fall back to cargo on failure
        if command -v cargo-near &> /dev/null; then
            echo -e "${YELLOW}   Trying cargo-near for optimized build...${NC}"
            if cargo near build non-reproducible-wasm; then
                # cargo-near puts WASM in target/near/ using the actual crate name
                # Get the actual crate name from Cargo.toml
                crate_name=$(grep "^name = " Cargo.toml | sed 's/name = "\(.*\)"/\1/' | tr '-' '_')
                
                if [ -f "target/near/${crate_name}.wasm" ]; then
                    cp "target/near/${crate_name}.wasm" "../../res/${contract_name}.wasm"
                elif [ -f "target/wasm32-unknown-unknown/release/${crate_name}.wasm" ]; then
                    cp "target/wasm32-unknown-unknown/release/${crate_name}.wasm" "../../res/${contract_name}.wasm"
                else
                    echo -e "${YELLOW}   WASM not found in expected locations, checking all files...${NC}"
                    # List available files for debugging
                    ls -la target/near/ 2>/dev/null || echo "target/near/ not found"
                    ls -la target/wasm32-unknown-unknown/release/*.wasm 2>/dev/null || echo "No WASM files in release/"
                fi
                echo -e "${GREEN}   cargo-near build successful${NC}"
            else
                echo -e "${YELLOW}   cargo-near failed, falling back to standard cargo...${NC}"
                CARGO_TARGET_DIR=./target cargo build --target wasm32-unknown-unknown --release
                
                # Get the actual crate name from Cargo.toml
                crate_name=$(grep "^name = " Cargo.toml | sed 's/name = "\(.*\)"/\1/' | tr '-' '_')
                
                if [ -f "target/wasm32-unknown-unknown/release/${crate_name}.wasm" ]; then
                    cp "target/wasm32-unknown-unknown/release/${crate_name}.wasm" "../../res/${contract_name}.wasm"
                fi
            fi
        else
            echo -e "${YELLOW}   Using standard cargo build...${NC}"
            CARGO_TARGET_DIR=./target cargo build --target wasm32-unknown-unknown --release
            
            # Get the actual crate name from Cargo.toml
            crate_name=$(grep "^name = " Cargo.toml | sed 's/name = "\(.*\)"/\1/' | tr '-' '_')
            
            if [ -f "target/wasm32-unknown-unknown/release/${crate_name}.wasm" ]; then
                cp "target/wasm32-unknown-unknown/release/${crate_name}.wasm" "../../res/${contract_name}.wasm"
            fi
        fi
        
        # Verify WASM was created
        if [ -f "../../res/${contract_name}.wasm" ]; then
            size=$(du -h "../../res/${contract_name}.wasm" | cut -f1)
            echo -e "${GREEN}   ✅ Built successfully: ${contract_name}.wasm ($size)${NC}"
        else
            echo -e "${RED}   ❌ WASM file not found for $contract_name${NC}"
            cd - > /dev/null
            exit 1
        fi
        
        cd - > /dev/null
        echo ""
    done
    
    echo -e "${GREEN}🎉 All contracts built successfully!${NC}"
}

# Optimize WASM files
optimize_wasm() {
    echo -e "${BLUE}⚡ Optimizing WASM files...${NC}"
    
    # Check if wasm-opt is available
    if command -v wasm-opt &> /dev/null; then
        for wasm_file in res/*.wasm; do
            echo "Optimizing $(basename $wasm_file)..."
            wasm-opt -Oz --output "$wasm_file" "$wasm_file"
        done
        echo -e "${GREEN}✅ WASM optimization complete${NC}"
    else
        echo -e "${YELLOW}⚠️ wasm-opt not found. Install binaryen for smaller WASM files.${NC}"
        echo "   npm install -g binaryen"
    fi
}

# Generate deployment manifest and show results
show_build_results() {
    echo -e "${BLUE}📊 Build Results:${NC}"
    echo "==================="
    
    # Create deployment manifest
    cat > deployment-manifest.json << EOF
{
  "system": "intent-based-prediction-market",
  "version": "1.0.0", 
  "build_time": "$(date -u +%Y-%m-%dT%H:%M:%SZ)",
  "architecture": "polymarket-ctf-near-intent",
  "contracts": {
    "ctf": {
      "name": "ConditionalTokenFramework",
      "description": "Polymarket-style CTF for position management",
      "wasm": "res/ctf.wasm",
      "dependencies": []
    },
    "resolver": {
      "name": "MarketResolver", 
      "description": "Manual market resolution with dispute mechanism",
      "wasm": "res/resolver.wasm",
      "dependencies": ["ctf"]
    },
    "verifier": {
      "name": "PredictionVerifier",
      "description": "Intent validation and cross-chain bridge",
      "wasm": "res/verifier.wasm", 
      "dependencies": ["ctf", "resolver"]
    },
    "solver": {
      "name": "PredictionSolver",
      "description": "Intent execution and CTF integration",
      "wasm": "res/solver.wasm",
      "dependencies": ["ctf", "verifier"]
    }
  },
  "deployment_order": ["ctf", "resolver", "verifier", "solver"]
}
EOF

    for wasm_file in res/*.wasm; do
        if [ -f "$wasm_file" ]; then
            size=$(du -h "$wasm_file" | cut -f1)
            hash=$(sha256sum "$wasm_file" | cut -d' ' -f1 | head -c 8)
            echo "✅ $(basename $wasm_file): $size (hash: $hash...)"
        fi
    done
    
    echo ""
    echo -e "${GREEN}🎉 Intent-Based Prediction Market Built Successfully!${NC}"
    echo -e "${BLUE}📄 Generated: deployment-manifest.json${NC}"
    echo -e "${BLUE}🚀 Ready for deployment to NEAR testnet${NC}"
}

# Main execution
main() {
    echo "Intent-Based Prediction Market - Build Process"
    echo "Polymarket CTF + NEAR Intent Integration"
    echo ""
    
    check_prerequisites
    clean_build
    build_contracts
    optimize_wasm
    show_build_results
    
    echo ""
    echo -e "${GREEN}🚀 Next Steps:${NC}"
    echo "1. Deploy contracts: ${YELLOW}./deploy.sh testnet${NC}"
    echo "2. Configure integration: ${YELLOW}./configure.sh${NC}" 
    echo "3. Run end-to-end tests: ${YELLOW}./test.sh${NC}"
    echo "4. Monitor system: ${YELLOW}./status.sh${NC}"
    echo ""
    echo -e "${BLUE}📖 Architecture Overview:${NC}"
    echo "   CTF: Polymarket-style conditional token framework"
    echo "   Resolver: Manual market resolution with disputes"
    echo "   Verifier: Intent validation + cross-chain bridge"
    echo "   Solver: Intent execution + CTF integration"
}

# Run main function
main "$@"