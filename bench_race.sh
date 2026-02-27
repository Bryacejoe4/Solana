#!/usr/bin/env bash
set -euo pipefail

# Load environment if present (relative to script location)
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
if [[ -f "$SCRIPT_DIR/.env.bench" ]]; then
  # shellcheck disable=SC1091
  source "$SCRIPT_DIR/.env.bench"
fi

API="${API:-http://127.0.0.1:8080}"
RPC="${RPC:-https://api.mainnet-beta.solana.com}"
ROUNDS="${ROUNDS:-10}"
SLEEP_MS="${SLEEP_MS:-120}"

BLOOM_AUTH_TOKEN="${BLOOM_AUTH_TOKEN:-}"
BLOOM_WALLET_ADDRESS="${BLOOM_WALLET_ADDRESS:-}"
BLOOM_URL="${BLOOM_URL:-https://us1.bloom-ext.app/api/extension-swap}"

# ---------- Bloom helper function ----------
# Uses a proper function to avoid all shell escaping issues
bloom_buy() {
  local mint="$1"
  local payload
  payload=$(cat <<EOF
{
  "id": "QT-${RANDOM}${RANDOM}-bench",
  "auth_token": "${BLOOM_AUTH_TOKEN}",
  "address": "${mint}",
  "amount": 0.01,
  "priority_fee": 0.002,
  "processor_tip": 0.01,
  "slippage": 40,
  "side": "Buy",
  "skip_if_bought": false,
  "anti_mev": true,
  "auto_tip": false,
  "dev_sell": null,
  "min_liquidity": 3000,
  "max_market_cap": 10000000,
  "amount_type": "exact_in",
  "wallets": [{"address": "${BLOOM_WALLET_ADDRESS}", "label": "W1"}]
}
EOF
  )
  curl -s -X POST "${BLOOM_URL}" \
    -H "Content-Type: application/json" \
    -d "${payload}"
}

# ---------- helpers ----------
get_sig() {
  sed -n 's/.*"signature"[[:space:]]*:[[:space:]]*"\([^"]*\)".*/\1/p'
}

sig_status() {
  local sig="$1"
  curl -s -X POST "$RPC" -H "Content-Type: application/json" -d "{
    \"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"getSignatureStatuses\",
    \"params\":[[\"$sig\"], {\"searchTransactionHistory\": true}]
  }"
}

is_confirmed() {
  local json="$1"
  echo "$json" | grep -q '"confirmationStatus":"confirmed"\|"confirmationStatus":"finalized"'
}

now_ms() { date +%s%3N; }

# ---------- pre-flight ----------
USE_BLOOM_API=false
if [[ -n "$BLOOM_AUTH_TOKEN" && -n "$BLOOM_WALLET_ADDRESS" ]]; then
  USE_BLOOM_API=true
  echo "[bench] Using official Bloom API (${BLOOM_URL})"
  echo "[bench] Warming up Bloom connection..."
  bloom_buy "11111111111111111111111111111111" > /dev/null 2>&1 || true
  sleep 1
else
  echo "WARNING: BLOOM_AUTH_TOKEN or BLOOM_WALLET_ADDRESS not set."
  echo "Simulating Bloom with 500ms latency..."
fi

# Use a real pump.fun token for the race
RACE_MINT="${RACE_MINT:-7EYnhQoR9YM3N7ebhc9ndS21ST6f67shf89A9f2S1Lp}"

wins_api=0
wins_bloom=0

echo "[bench] API=$API"
echo "[bench] RPC=$RPC"
echo "[bench] ROUNDS=$ROUNDS"
echo "[bench] MINT=$RACE_MINT"
echo

for i in $(seq 1 "$ROUNDS"); do
  memo="race-$i-$(date +%s)"
  t0=$(now_ms)

  api_out_file="$(mktemp)"
  bloom_out_file="$(mktemp)"

  # Fire API
  (
    curl -s -X POST "$API/v1/solana/memo_fast" \
      -H "Content-Type: application/json" \
      -d "{\"memo\":\"$memo\"}" > "$api_out_file"
  ) &

  # Fire Bloom (or simulation)
  (
    if $USE_BLOOM_API; then
      bloom_buy "$RACE_MINT" > "$bloom_out_file" 2>&1
    else
      sleep 0.5
      curl -s -X POST "$API/v1/solana/memo_fast" \
        -H "Content-Type: application/json" \
        -d "{\"memo\":\"bloom-$memo\"}" > "$bloom_out_file"
    fi
  ) &

  wait

  api_sig="$(cat "$api_out_file" | get_sig || true)"
  bloom_sig="$(cat "$bloom_out_file" | get_sig || true)"

  if [[ -z "$api_sig" ]]; then
    echo "[$i] API failure! Response:"
    cat "$api_out_file"; echo
  fi

  if [[ -z "$bloom_sig" ]]; then
    echo "[$i] BLOOM failure! Response:"
    cat "$bloom_out_file"; echo
  fi

  if [[ -z "$api_sig" || -z "$bloom_sig" ]]; then
    echo "[$i] Skipping."
    echo
    continue
  fi

  # Poll confirmations
  api_t=""
  bloom_t=""

  while [[ -z "$api_t" || -z "$bloom_t" ]]; do
    if [[ -z "$api_t" ]]; then
      js="$(sig_status "$api_sig")"
      if is_confirmed "$js"; then api_t=$(now_ms); fi
    fi
    if [[ -z "$bloom_t" ]]; then
      js="$(sig_status "$bloom_sig")"
      if is_confirmed "$js"; then bloom_t=$(now_ms); fi
    fi
    python3 - <<PY >/dev/null 2>&1 || sleep 0.12
import time; time.sleep(${SLEEP_MS}/1000)
PY
  done

  api_dt=$((api_t - t0))
  bloom_dt=$((bloom_t - t0))

  if (( api_dt < bloom_dt )); then
    winner="API"
    wins_api=$((wins_api+1))
  elif (( bloom_dt < api_dt )); then
    winner="BLOOM"
    wins_bloom=$((wins_bloom+1))
  else
    winner="TIE"
  fi

  echo "[$i] winner=$winner  api=${api_dt}ms  bloom=${bloom_dt}ms"
  echo "    api_sig=$api_sig"
  echo "    bloom_sig=$bloom_sig"
  echo
done

echo "=== SUMMARY ==="
echo "API wins:   $wins_api"
echo "BLOOM wins: $wins_bloom"
echo "Ties/other: $((ROUNDS - wins_api - wins_bloom))"
