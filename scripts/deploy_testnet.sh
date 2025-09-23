#!/usr/bin/env bash
set -euo pipefail

# Deploy Prediction Marketplace contracts to NEAR testnet
# Contracts (same WASM, different init):
# - ctf::ConditionalTokenFramework
# - verifier::PredictionVerifier
# - solver::PredictionSolver
# - resolver::MarketResolver

# Prereqs:
# - near CLI installed: https://near.cli.rs
# - cargo-near installed: https://github.com/near/cargo-near
# - Logged in to testnet: near account import or near login

# Usage:
#   ./scripts/deploy_testnet.sh \
#     --root-account yourroot.testnet \
#     --owner-account owner.yourroot.testnet \
#     --usdc-account usdc.fakes.testnet \
#       or pass --create-mock-usdc to deploy a mock token at <prefix>-usdc.<root>
#     --orderbook-authority svc.yourroot.testnet \
#     [--prefix pmkt] \
#     [--initial-balance 10] \
#     [--network testnet]
#
# It will create and/or reuse the following accounts:
#   <prefix>-ctf.<root>
#   <prefix>-verifier.<root>
#   <prefix>-solver.<root>
#   <prefix>-resolver.<root>

ROOT_ACCOUNT=""
OWNER_ACCOUNT=""
USDC_ACCOUNT=""
ORDERBOOK_AUTH=""
PREFIX="pmkt"
INITIAL_BALANCE_NEAR="10"
NETWORK="testnet"
CREATE_MOCK_USDC="false"

while [[ $# -gt 0 ]]; do
  case "$1" in
    --root-account)
      ROOT_ACCOUNT="$2"; shift; shift ;;
    --owner-account)
      OWNER_ACCOUNT="$2"; shift; shift ;;
    --usdc-account)
      USDC_ACCOUNT="$2"; shift; shift ;;
    --orderbook-authority)
      ORDERBOOK_AUTH="$2"; shift; shift ;;
    --prefix)
      PREFIX="$2"; shift; shift ;;
    --initial-balance)
      INITIAL_BALANCE_NEAR="$2"; shift; shift ;;
    --network)
      NETWORK="$2"; shift; shift ;;
    --create-mock-usdc)
      CREATE_MOCK_USDC="true"; shift ;;
    *) echo "Unknown arg: $1"; exit 1 ;;
  esac
done

if [[ -z "$ROOT_ACCOUNT" || -z "$OWNER_ACCOUNT" || -z "$ORDERBOOK_AUTH" ]]; then
  echo "Missing required args. See file header for usage." >&2
  exit 1
fi

if ! command -v near >/dev/null 2>&1; then
  echo "near CLI not found. Install from https://near.cli.rs" >&2
  exit 1
fi
if ! command -v cargo >/dev/null 2>&1; then
  echo "cargo not found (Rust toolchain)." >&2
  exit 1
fi
if ! command -v cargo near >/dev/null 2>&1; then
  echo "cargo-near not found. Install: cargo install cargo-near" >&2
  exit 1
fi

SCRIPT_DIR=$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)
PROJECT_DIR=$(cd "$SCRIPT_DIR/.." && pwd)
cd "$PROJECT_DIR"

echo "Building WASM (reproducible)..."
# You can switch to build-non-reproducible-wasm if needed for speed during dev
cargo near build build-reproducible-wasm

# Try common output locations
WASM=""
if [[ -f "target/near/release/prediction_marketplace.wasm" ]]; then
  WASM="target/near/release/prediction_marketplace.wasm"
elif [[ -f "res/prediction_marketplace.wasm" ]]; then
  WASM="res/prediction_marketplace.wasm"
else
  # Fallback to find
  WASM=$(find target res -type f -name 'prediction_marketplace.wasm' 2>/dev/null | head -n1 || true)
fi

if [[ -z "$WASM" || ! -f "$WASM" ]]; then
  echo "Could not locate built WASM. Looked under target/near/release and res/." >&2
  exit 1
fi

echo "Using WASM: $WASM"

CTF_ACCOUNT="${PREFIX}-ctf.${ROOT_ACCOUNT}"
VERIFIER_ACCOUNT="${PREFIX}-verifier.${ROOT_ACCOUNT}"
SOLVER_ACCOUNT="${PREFIX}-solver.${ROOT_ACCOUNT}"
RESOLVER_ACCOUNT="${PREFIX}-resolver.${ROOT_ACCOUNT}"
MOCK_USDC_ACCOUNT="${PREFIX}-usdc.${ROOT_ACCOUNT}"

create_if_missing() {
  local acct="$1"
  local initial_near="$2"
  set +e
  near account view "$acct" --networkId "$NETWORK" >/dev/null 2>&1
  local exists=$?
  set -e
  if [[ $exists -ne 0 ]]; then
    echo "Creating account $acct"
    near account create-account "$acct" \
      --networkId "$NETWORK" \
      --useFaucet \
      --initialBalance "$initial_near" \
      --parentAccountId "$ROOT_ACCOUNT"
  else
    echo "Account $acct exists. Reusing."
  fi
}

echo "Ensuring subaccounts exist..."
create_if_missing "$CTF_ACCOUNT" "$INITIAL_BALANCE_NEAR"
create_if_missing "$VERIFIER_ACCOUNT" "$INITIAL_BALANCE_NEAR"
create_if_missing "$SOLVER_ACCOUNT" "$INITIAL_BALANCE_NEAR"
create_if_missing "$RESOLVER_ACCOUNT" "$INITIAL_BALANCE_NEAR"

# Optionally create a mock USDC token account if requested and no explicit USDC provided
if [[ "$CREATE_MOCK_USDC" == "true" && -z "$USDC_ACCOUNT" ]]; then
  echo "Creating mock USDC account at $MOCK_USDC_ACCOUNT"
  create_if_missing "$MOCK_USDC_ACCOUNT" "$INITIAL_BALANCE_NEAR"
  USDC_ACCOUNT="$MOCK_USDC_ACCOUNT"
fi

echo "Deploying WASM to accounts..."
near contract deploy "$CTF_ACCOUNT" --wasmFile "$WASM" --networkId "$NETWORK"
near contract deploy "$VERIFIER_ACCOUNT" --wasmFile "$WASM" --networkId "$NETWORK"
near contract deploy "$SOLVER_ACCOUNT" --wasmFile "$WASM" --networkId "$NETWORK"
near contract deploy "$RESOLVER_ACCOUNT" --wasmFile "$WASM" --networkId "$NETWORK"

# If using mock USDC, deploy to that account as well
if [[ "$CREATE_MOCK_USDC" == "true" && "$USDC_ACCOUNT" == "$MOCK_USDC_ACCOUNT" ]]; then
  near contract deploy "$USDC_ACCOUNT" --wasmFile "$WASM" --networkId "$NETWORK"
fi

# Initialize contracts
# If using mock USDC, initialize it first so its account ID is valid for CTF and Solver
if [[ "$CREATE_MOCK_USDC" == "true" && "$USDC_ACCOUNT" == "$MOCK_USDC_ACCOUNT" ]]; then
  echo "Initializing Mock USDC token..."
  # name: "USDC Mock", symbol: "USDC", decimals: 6, initial_supply: 1e15 (1,000,000,000,000,000)
  near contract call "$USDC_ACCOUNT" new \
    '{"owner_id":"'"$OWNER_ACCOUNT"'","name":"USDC Mock","symbol":"USDC","decimals":6,"initial_supply":"1000000000000000"}' \
    --accountId "$OWNER_ACCOUNT" --networkId "$NETWORK" --gas 100000000000000 --deposit 0
fi
# ctf::ConditionalTokenFramework::new(owner_id, usdc_contract)
echo "Initializing CTF..."
near contract call "$CTF_ACCOUNT" new \
  '{"owner_id":"'"$OWNER_ACCOUNT"'","usdc_contract":"'"$USDC_ACCOUNT"'"}' \
  --accountId "$OWNER_ACCOUNT" --networkId "$NETWORK" --gas 300000000000000 --deposit 0

# resolver::MarketResolver::new(owner_id, verifier_contract, ctf_contract, dispute_period_ns, dispute_bond_yocto)
# Defaults: dispute period 24h, bond 1 NEAR
dispute_period_ns=$((24*60*60*1000000000))
dispute_bond_yocto="1000000000000000000000000"
echo "Initializing Resolver..."
near contract call "$RESOLVER_ACCOUNT" new \
  '{"owner_id":"'"$OWNER_ACCOUNT"'","verifier_contract":"'"$VERIFIER_ACCOUNT"'","ctf_contract":"'"$CTF_ACCOUNT"'","dispute_period":'"$dispute_period_ns"',"dispute_bond":"'"$dispute_bond_yocto"'"}' \
  --accountId "$OWNER_ACCOUNT" --networkId "$NETWORK" --gas 300000000000000 --deposit 0

# verifier::PredictionVerifier::new(owner_id, ctf_contract, resolver_contract, min_bet_amount, max_bet_amount, platform_fee_bps)
# Defaults: min 1e6 (1 USDC with 6 decimals), max 1e12, fee 100 (1%)
echo "Initializing Verifier..."
near contract call "$VERIFIER_ACCOUNT" new \
  '{"owner_id":"'"$OWNER_ACCOUNT"'","ctf_contract":"'"$CTF_ACCOUNT"'","resolver_contract":"'"$RESOLVER_ACCOUNT"'","min_bet_amount":"1000000","max_bet_amount":"1000000000000","platform_fee_bps":100}' \
  --accountId "$OWNER_ACCOUNT" --networkId "$NETWORK" --gas 300000000000000 --deposit 0

# solver::PredictionSolver::new(owner_id, verifier_contract, ctf_contract, usdc_contract, orderbook_authority, solver_fee_bps, min_order_size)
# Defaults: fee 50 (0.5%), min order 1e6
echo "Initializing Solver..."
near contract call "$SOLVER_ACCOUNT" new \
  '{"owner_id":"'"$OWNER_ACCOUNT"'","verifier_contract":"'"$VERIFIER_ACCOUNT"'","ctf_contract":"'"$CTF_ACCOUNT"'","usdc_contract":"'"$USDC_ACCOUNT"'","orderbook_authority":"'"$ORDERBOOK_AUTH"'","solver_fee_bps":50,"min_order_size":"1000000"}' \
  --accountId "$OWNER_ACCOUNT" --networkId "$NETWORK" --gas 300000000000000 --deposit 0

# Wire: register solver in verifier
echo "Registering solver in verifier..."
near contract call "$VERIFIER_ACCOUNT" register_solver '{"solver":"'"$SOLVER_ACCOUNT"'"}' \
  --accountId "$OWNER_ACCOUNT" --networkId "$NETWORK" --gas 100000000000000 --deposit 0

# Optional: authorize resolver/oracles in resolver (owner only)
# near contract call "$RESOLVER_ACCOUNT" add_oracle '{"oracle":"oracle.'"$ROOT_ACCOUNT"'"}' --accountId "$OWNER_ACCOUNT" --networkId "$NETWORK" --gas 100000000000000 --deposit 0

# Basic verification views
echo "Verifying deployments..."
near contract view "$VERIFIER_ACCOUNT" get_registered_solvers '{}' --networkId "$NETWORK"
near contract view "$VERIFIER_ACCOUNT" get_platform_config '{}' --networkId "$NETWORK"

# Optional: Verify CTF contract
echo "CTF contract status:"
near contract view "$CTF_ACCOUNT" get_config '{}' --networkId "$NETWORK" || echo "CTF config check failed"

# Optional: Check mock USDC if deployed
if [[ "$CREATE_MOCK_USDC" == "true" && "$USDC_ACCOUNT" == "$MOCK_USDC_ACCOUNT" ]]; then
  echo "Mock USDC status:"
  near contract view "$USDC_ACCOUNT" ft_metadata '{}' --networkId "$NETWORK" || echo "USDC metadata check failed"
fi

cat <<EOF

✅ Deployment complete.
Contracts:
  CTF:       $CTF_ACCOUNT
  Verifier:  $VERIFIER_ACCOUNT
  Solver:    $SOLVER_ACCOUNT
  Resolver:  $RESOLVER_ACCOUNT
EOF

if [[ "$CREATE_MOCK_USDC" == "true" && "$USDC_ACCOUNT" == "$MOCK_USDC_ACCOUNT" ]]; then
cat <<EOF
  USDC:      $USDC_ACCOUNT (Mock Token)
EOF
else
cat <<EOF
  USDC:      $USDC_ACCOUNT (External Token)
EOF
fi

cat <<EOF

🔧 Configuration:
  Owner:              $OWNER_ACCOUNT
  Orderbook Authority: $ORDERBOOK_AUTH
  Network:            $NETWORK
  Prefix:             $PREFIX

📋 Next steps:
  1. Create a market via verifier.create_market(...)
  2. Submit an intent and run verify_and_solve(...)
  3. Use resolver to finalize and trigger CTF payouts
  4. Start orderbook service: cd orderbook-service && cargo run

💡 Testing commands:
  # View registered solvers
  near contract view $VERIFIER_ACCOUNT get_registered_solvers '{}' --networkId $NETWORK
  
  # Create test market
  near contract call $VERIFIER_ACCOUNT create_market \\
    '{"question":"Will BTC reach 100k by 2024?","outcomes":["NO","YES"],"end_time":'"$(($(date +%s) + 86400))"'000000000,"initial_liquidity":"1000000000"}' \\
    --accountId $OWNER_ACCOUNT --networkId $NETWORK --gas 300000000000000 --deposit 1000000000000000000000000

🚀 Ready for testing!
EOF
