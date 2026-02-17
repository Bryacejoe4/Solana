# 🧪 Solana Sniper Client (TS Reference Variant)

This is a **TypeScript-based reference client** and variant for the Solana Sniper project. While the Major Project (in Rust) handles high-performance execution, this folder provides a flexible environment for rapid prototyping and detection logic.

## 🚀 Role in Project
- **Reference Logic**: Contains the initial implementation of Pump.fun bonding curve logic.
- **Client Implementation**: Can be used as a front-end/client to interact with the Rust execution engine.
- **Variant Execution**: Uses Helius Smart Sender and Jito Bundles for optimized TS execution.

## 📂 Features
- **Helius Integration**: Smart routing via Jito/Helius RPC.
- **Detection**: gRPC streaming for new token launches (Pump.fun).
- **Position Management**: TP/SL logic in JS/TS.

## 🛠 Setup
1. pm install2. pm start
---
*The primary execution engine for production is located in the \solana-sniper\ (Rust) folder.*
