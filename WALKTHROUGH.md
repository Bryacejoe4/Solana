# 📘 Solana High-Performance Sniper: The Complete Client Manual

This manual provides the **exact commands** to execute every requirement in your specification: buying, selling, benchmarking against Bloom, and running the API.

---

## 🛠️ Part 1: Installation & Setup

1.  **Install Dependencies (WSL)**:
    ```bash
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    source $HOME/.cargo/env
    sudo apt update && sudo apt install -y build-essential pkg-config libssl-dev git
    ```

2.  **Clone & Configure**:
    ```bash
    cd /home/praise/solana-sniper
    cp config/config.example.toml config/config.toml
    ```

3.  **Edit Config**:
    Run `nano config/config.toml` and fill in:
    *   `rpc_url`: `https://mainnet.helius-rpc.com/?api-key=YOUR_KEY`
    *   `ws_url`: `wss://mainnet.helius-rpc.com/?api-key=YOUR_KEY`
    *   `private_key`: Your **Base58** Private Key.
    *   `jito_auth_keypair`: (Optional) `[123, 45, ...]` for Jito auth.
    *   `slippage_bps`: `1000` (10%).

---

## 🚀 Part 2: Starting the Execution Engine

This starts the high-performance Rust server that listens for trade signals.

```bash
cd /home/praise/solana-sniper
cargo run -p api --release
```
*   **Success**: You will see `Listening on 0.0.0.0:8080`.
*   **Background**: Leave this terminal running. Open a **new terminal** for the commands below.

---

## 💰 Part 3: Live Trading Commands (Pump.fun)

The specification requires **Buy**, **Sell**, and **Abort** functionality. You trigger these via local API calls (simulating a frontend).

### 1. Buy Command (Market Buy on Pump.fun)
Executes a swap from SOL to Token.
```bash
curl -X POST http://localhost:8080/v1/trade/buy \
  -H "Content-Type: application/json" \
  -d '{
    "launchpad": "pumpfun",
    "token_mint": "7EYnhQoR9YM3N7ebhc...MINT_ADDRESS...",
    "amount_sol": 0.1,
    "max_slippage_bps": 500,
    "priority_fee_lamports": 100000,
    "jito_tip_lamports": 100000
  }'
```
*   **Expected Output**: `{"status": "submitted", "signature": "5Kt..."}`
*   **On-Chain**: Use a block explorer (e.g., Solscan) to verify the tx was landed by Jito.

### 2. Sell Command (Exit Position)
Executes a swap from Token to SOL.
```bash
curl -X POST http://localhost:8080/v1/trade/sell \
  -H "Content-Type: application/json" \
  -d '{
    "launchpad": "pumpfun",
    "token_mint": "7EYnhQoR9YM3N7ebhc...MINT_ADDRESS...",
    "amount_tokens": 1000000,
    "min_sol_output": 0.05
  }'
```

### 3. Abort/Cancel (Mid-Execution)
If a trade is stuck or conditions change (per spec), call the cancel endpoint.
```bash
curl -X POST http://localhost:8080/v1/trade/cancel \
  -H "Content-Type: application/json" \
  -d '{"token_mint": "..."}'
```

---

## 🏎️ Part 4: Benchmarking Against Bloom

Your specification demands proof that this engine is faster than Bloom. We use the `bench_race.sh` script to verify this.

1.  **Prepare the Race**:
    Go to the home folder:
    ```bash
    cd /home/praise/
    chmod +x bench_race.sh
    ```

2.  **Set the Bloom Competitor**:
    You need a valid Bloom API command to race against.
    ```bash
    export BLOOM_CMD='curl -s -X POST https://bloom-api.example.com/v1/swap -H "Authorization: KEY" -d "..."'
    ```

3.  **Run the Benchmark**:
    ```bash
    ./bench_race.sh
    ```
    *   **The Script**: Fires 10 parallel trades to both systems.
    *   **The Check**: Queries the Solana RPC to find which transaction was confirmed in an earlier slot.
    *   **The Result**: Prints `WINNER: API (Rust)` when your engine lands faster.

---

## 📊 Part 5: "Real Transactions" Verification

To verify **real execution** without losing money:

1.  **Use Small Amounts**: Set `amount_sol` to `0.0001` in the Buy command.
2.  **Target a Stable Token**: Use a token with high liquidity to avoid slippage fails.
3.  **Check the Logs**:
    The Rust terminal will show:
    > `INFO [Executor] Building Buy Instruction...`
    > `INFO [Simulation] Simulating locally...`
    > `INFO [Jito] Bundle sent to 4 regions.`
    > `INFO [Confirm] Confirmed in slot 298123012`

This log sequence proves **local build -> simulation -> direct submission** as requested.

---
---

## 📦 Part 6: Delivery & GitHub Setup

To deliver this to your client via GitHub, follow these exact steps:

1.  **Prepare the Clean Repository**:
    ```bash
    cd /home/praise/solana-sniper
    # Remove local target files and sensitive configs
    cargo clean
    rm config/config.toml
    ```

2.  **Push to GitHub**:
    ```bash
    git add .
    git commit -m "feat: implement high-performance rust execution engine v1 (Phase 1)"
    git push origin main
    ```

3.  **Client Running Instructions (The instructions you give to him)**:
    Tell your client to follow these steps once they have access to the repo:
    1.  `git clone <REPO_URL>`
    2.  `cd solana-sniper`
    3.  `curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh`
    4.  `cargo build --release`
    5.  `cp config/config.example.toml config/config.toml` (Then edit with their RPC/Key)
    6.  `cargo run -p api --release`

---
*Prepared for Client Review - Phase 1 Complete*
