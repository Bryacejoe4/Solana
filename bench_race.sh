#!/usr/bin/env bash
set -euo pipefail

API="${API:-http://127.0.0.1:8080}"
RPC="${RPC:-https://api.mainnet-beta.solana.com}"
ROUNDS="${ROUNDS:-10}"
SLEEP_MS="${SLEEP_MS:-120}"   # polling interval

# You MUST set BLOOM_CMD to a command that prints JSON containing a "signature" field.
# Example:
# export BLOOM_CMD='curl -s -X POST https://bloom... -H "Content-Type: application/json" -d "{\"memo\":\"race\"}"'
BLOOM_CMD="${BLOOM_CMD:-}"

if [[ -z "$BLOOM_CMD" ]]; then
  echo "ERROR: BLOOM_CMD is not set."
  echo "Set it like:"
  echo "  export BLOOM_CMD='curl -s -X POST https://... -H \"Content-Type: application/json\" -d \"{...}\"'"
  exit 1
fi

get_sig() {
  # Extract `"signature":"..."` from JSON (no jq dependency)
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
  # returns 0 if confirmed/finalized, else 1
  local json="$1"
  echo "$json" | grep -q '"confirmationStatus":"confirmed"\|"confirmationStatus":"finalized"'
}

now_ms() { date +%s%3N; }

wins_api=0
wins_bloom=0

echo "[bench] API=$API"
echo "[bench] RPC=$RPC"
echo "[bench] ROUNDS=$ROUNDS"
echo

for i in $(seq 1 "$ROUNDS"); do
  memo="race-$i-$(date +%s)"

  t0=$(now_ms)

  # Fire API and Bloom in parallel and capture output
  api_out_file="$(mktemp)"
  bloom_out_file="$(mktemp)"

  (
    curl -s -X POST "$API/v1/solana/memo_fast" \
      -H "Content-Type: application/json" \
      -d "{\"memo\":\"$memo\"}" > "$api_out_file"
  ) &

  (
    bash -lc "$BLOOM_CMD" > "$bloom_out_file"
  ) &

  wait

  api_sig="$(cat "$api_out_file" | get_sig || true)"
  bloom_sig="$(cat "$bloom_out_file" | get_sig || true)"

  if [[ -z "$api_sig" ]]; then
    echo "[$i] API did not return signature. Output:"
    cat "$api_out_file"; echo
    continue
  fi

  if [[ -z "$bloom_sig" ]]; then
    echo "[$i] BLOOM did not return signature. Output:"
    cat "$bloom_out_file"; echo
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

    # sleep in ms (works on GNU sleep with seconds; fallback)
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
