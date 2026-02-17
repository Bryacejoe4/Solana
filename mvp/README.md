# 🎯 Solana Sniper MVP (Phase 1 Variant)

This is the **MVP (Proof of Capability)** version of the Solana Sniper. It focuses on validating execution quality and benchmarking against third-party services like Bloom.

## 🚀 MVP Scope
- **Target Launchpads**: Pump.fun, Pump.fun AMM, Bags.
- **Functionality**: Buy/Sell execution with configurable slippage and priority fees.
- **Submission**: Parallel submission to Jito and NextBlock.

## 📂 Differences from Major Project
- Simplified configuration for rapid testing.
- Focused exclusively on Phase 1 requirements.
- Used for A/B testing against Bloom API latency.

## 🛠 Quick Start
1. \cp config/config.example.toml config/config.toml2. \cargo run -p api
---
*Success is measured by execution speed and reliability, not feature count.*
