# ⚡ Solana Sniper: High-Performance Execution Manual

Hey, as we discussed, this is the Phase 1 delivery of the custom Solana execution engine. The goal of this system is to give us a definitive edge by completely bypassing third-party APIs like Bloom and interacting directly with the blockchain.

Every millisecond counts in sniping, so I've built this in **Rust** to ensure zero-latency execution and maximum control.

---

## 🛠️ 1. Environment & Setup

I’ve optimized this to run natively on Linux (WSL/Ubuntu). This ensures we have the lowest possible overhead for network calls and transaction processing.

### **Server Installation**
```bash
# Install the Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env

# Install critical system libraries
sudo apt update && sudo apt install -y build-essential pkg-config libssl-dev git
```

### **Building the Engine**
We compile the code locally to ensure it's specifically tuned for our hardware.
```bash
cd solana-sniper
cargo build --release
```

---

## 🚀 2. Why This is Faster Than Bloom (The Proof)

The primary bottleneck in Bloom is the network hop to their API and their internal processing time. My system fixes this by:

1.  **Local Instruction Building**: We don't ask an API how to trade. We build the instructions ourselves.
    *   *Check file:* `strategies/src/pumpfun.rs`
2.  **Native Simulation**: We simulate the trade locally to ensure it will land before we even send it.
    *   *Check file:* `executor/src/engine.rs`
3.  **Direct Jito Submission**: We send our transaction "bundles" directly to Jito's block engines, bypassing the slow public mempool.
    *   *Check file:* `mev/src/jito.rs`

---

## 💰 3. Running the Engine & Live Trading

### **Step A: Start the Engine**
This starts the high-speed execution listener.
```bash
# Set up your config first (Add your Private Key and RPC)
cp config/config.example.toml config/config.toml
nano config/config.toml

# Launch the engine
cargo run -p api --release
```

### **Step B: Market Execution (Buy/Sell)**
Use these commands in a separate terminal to trigger the engine.

**Market Buy (Pump.fun)**
```bash
curl -X POST http://localhost:8080/v1/trade/buy \
  -H "Content-Type: application/json" \
  -d '{
    "launchpad": "pumpfun",
    "token_mint": "7EYnhQoR9YM3N7ebhc9ndS21ST6f67shf89A9f2S1Lp",
    "amount_sol": 0.01,
    "max_slippage_bps": 500
  }'
```

**Market Sell (Exit Position)**
```bash
curl -X POST http://localhost:8080/v1/trade/sell \
  -H "Content-Type: application/json" \
  -d '{
    "launchpad": "pumpfun",
    "token_mint": "7EYnhQoR9YM3N7ebhc9ndS21ST6f67shf89A9f2S1Lp",
    "amount_tokens": 1000000
  }'
```

---

## 🏎️ 4. Benchmarking: The "Race" vs Bloom

To prove that we are faster, I’ve included a benchmarking script. It fires a trade to both systems simultaneously and checks the Solana slot data to see who landed first.

```bash
# Set your Bloom API key
export BLOOM_CMD='curl -s -X POST https://bloom-api.example.com/v1/swap -H "Authorization: YOUR_KEY"'

# Run the race
chmod +x bench_race.sh
./bench_race.sh
```
**Success Criteria**: When the custom engine lands in an earlier slot than Bloom, the script will output: `WINNER: API (Rust)`.

---

## 📈 Summary of Requirements Met

*   ✅ **High Performance**: Native Rust core.
*   ✅ **Direct Submission**: Integrated Jito Bundler.
*   ✅ **Native Logic**: local Pump.fun instruction building.
*   ✅ **Phase 1 Complete**: Buy, Sell, and Abort functions are live.
*   ✅ **Scalable**: Architecture is ready for Raydium and Meteora in Phase 2.

*Prepared for Client Review - Phase 1*
