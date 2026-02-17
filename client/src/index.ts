import { Connection, Keypair, PublicKey, LAMPORTS_PER_SOL, TransactionInstruction } from '@solana/web3.js';
import { ConnectionManager } from './engine/connection';
import { JitoClient } from './engine/jito';
import { TransactionBuilder } from './engine/transaction';
import { PumpFunStrategy } from './strategies/pumpfun';
import { loadKeypairFromEnv } from './utils/keys';
import { getDerivedAccounts } from './utils/pda';
import { SafetyChecker } from './engine/safety';
import { PositionManager } from './engine/position';

import { PriorityFeeManager } from './engine/priority';
import { Sniper } from './engine/sniper';
import { GrpcClient } from './engine/grpc';
import { HeliusSender } from './engine/helius';
import { BondingCurveManager } from './engine/bondingCurve';
import * as dotenv from 'dotenv';
import * as readline from 'readline';

dotenv.config();

const rl = readline.createInterface({
    input: process.stdin,
    output: process.stdout
});

async function main() {
    console.log("========================================");
    console.log("   SOLANA HIGH-PERFORMANCE SNIPER BOT   ");
    console.log("========================================");

    // 1. Setup
    const connection = ConnectionManager.getInstance().getConnection();

    let wallet: Keypair;
    try {
        wallet = loadKeypairFromEnv('PRIVATE_KEY');
    } catch (e) {
        console.error("Error: PRIVATE_KEY not found in .env");
        console.log("Please copy .env.example to .env and add your key.");
        process.exit(1);
    }

    const jito = JitoClient.getInstance();
    const positionManager = PositionManager.getInstance();

    console.log(`Wallet: ${wallet.publicKey.toBase58()}`);
    try {
        const balance = await connection.getBalance(wallet.publicKey);
        console.log(`Balance: ${(balance / LAMPORTS_PER_SOL).toFixed(4)} SOL`);
    } catch (e) {
        console.log("Could not fetch balance (RPC might be rate limited or down).");
    }

    // Start Position Monitoring in background
    setInterval(() => {
        positionManager.monitorPositions(async (mint, amount) => {
            console.log(`[Auto-Sell] Triggered for ${mint.toBase58()}`);

            // Re-use sell logic (refactored slightly to not rely on prompt)
            await executeSell(connection, wallet, jito, mint, amount);
        });
    }, 5000); // Check every 5 seconds

    // 2. Command Loop
    while (true) {
        console.log("\nCommands: [buy] [sell] [snipe] [exit]");
        const answer = await new Promise<string>(resolve => rl.question('Action: ', resolve));

        if (answer.trim() === 'exit') break;

        if (answer.trim() === 'buy') {
            await handleBuy(connection, wallet, jito);
        } else if (answer.trim() === 'sell') {
            await handleSell(connection, wallet, jito);
        } else if (answer.trim() === 'snipe') {
            await handleSnipe(connection, wallet, jito);
        }
    }

    rl.close();
    process.exit(0);
}

async function handleSnipe(connection: Connection, wallet: Keypair, jito: JitoClient) {
    const amountStr = await new Promise<string>(resolve => rl.question('Amount in SOL to snipe per token: ', resolve));
    const amountSol = parseFloat(amountStr);

    console.log("Target Mint Address (Optional - Press Enter to snipe ALL new tokens):");
    console.log("Example provided by user: a3W4qutoEJA4232T2gwZUfgYJTetr96pU4SJMwppump");
    const targetMintStr = await new Promise<string>(resolve => rl.question('Target Mint: ', resolve));
    const targetMint = targetMintStr.trim() ? new PublicKey(targetMintStr.trim()) : null;

    const sniper = new Sniper();
    const safety = SafetyChecker.getInstance();

    // Check if gRPC is enabled
    const useGrpc = process.env.GRPC_ENABLED === 'true';
    const grpcEndpoint = process.env.GRPC_ENDPOINT;
    const grpcToken = process.env.GRPC_TOKEN || "";

    console.log(`Waiting for new Pump.fun tokens... (Press Ctrl+C to stop logic, or exit via terminal)`);

    // Callback function for detection (shared by both engines)
    const onTokenDetected = async (signature: string, logs: string[]) => {
        console.log(`\n[!] New Token Detected! Sig: ${signature}`);

        // Fetch transaction to parse Mint address
        try {
            const tx = await connection.getParsedTransaction(signature, {
                maxSupportedTransactionVersion: 0,
                commitment: 'confirmed'
            });

            if (!tx || !tx.transaction || !tx.transaction.message) {
                console.log("Could not fetch transaction details.");
                return;
            }

            const accountKeys = tx.transaction.message.accountKeys;
            let detectedMint: PublicKey | null = null;

            if (targetMint) {
                // @ts-ignore
                const present = accountKeys.find((ak: any) => ak.pubkey.equals(targetMint));
                if (present) detectedMint = targetMint;
            } else {
                if (accountKeys.length > 2) {
                    // @ts-ignore
                    detectedMint = accountKeys[2].pubkey;
                }
            }

            if (detectedMint) {
                console.log(`Identified Mint: ${detectedMint.toBase58()}`);

                if (targetMint && !detectedMint.equals(targetMint)) {
                    console.log(`Ignoring ${detectedMint.toBase58()} (Not target)`);
                    return;
                }

                // SAFETY CHECK
                const isSafe = await safety.checkToken(detectedMint);
                if (!isSafe) {
                    console.log(`[Safety] Skipped ${detectedMint.toBase58()} (Failed checks)`);
                    return;
                }

                console.log(`>> AUTO-BUYING ${detectedMint.toBase58()}...`);
                await buyToken(connection, wallet, jito, detectedMint, amountSol);
            } else {
                console.log("Could not identify Mint account.");
            }

        } catch (e) {
            console.error("Error parsing detected transaction:", e);
        }
    };

    if (useGrpc && grpcEndpoint) {
        console.log(">>> MODE: gRPC STREAMING (Ultra-Fast) ⚡");
        const grpc = new GrpcClient(grpcEndpoint, grpcToken);
        await grpc.start(onTokenDetected);
    } else {
        console.log(">>> MODE: HTTP POLL (Standard Speed) 🐢");
        if (useGrpc) console.log("Warning: GRPC_ENABLED is true but GRPC_ENDPOINT is missing.");
        await sniper.startListener(onTokenDetected);
    }

    // Keep alive
    await new Promise(resolve => { });
}

async function handleBuy(connection: Connection, wallet: Keypair, jito: JitoClient) {
    const mintStr = await new Promise<string>(resolve => rl.question('Token Mint Address: ', resolve));
    const amountStr = await new Promise<string>(resolve => rl.question('Amount in SOL (e.g., 0.1): ', resolve));

    try {
        const mint = new PublicKey(mintStr);
        const amountSol = parseFloat(amountStr);
        await buyToken(connection, wallet, jito, mint, amountSol);
    } catch (e) {
        console.error("Invalid input:", e);
    }
}

async function buyToken(connection: Connection, wallet: Keypair, jito: JitoClient, mint: PublicKey, amountSol: number) {
    try {
        const amountLamports = BigInt(Math.floor(amountSol * LAMPORTS_PER_SOL));

        console.log("Deriving accounts/curve...");
        const { bondingCurve, associatedBondingCurve, userATA } = getDerivedAccounts(mint, wallet.publicKey);

        // Create Instruction
        // Slippage: 15%
        const maxSolCost = BigInt(Math.floor(Number(amountLamports) * 1.15));

        const ix = PumpFunStrategy.createBuyInstruction(
            wallet.publicKey,
            mint,
            bondingCurve,
            associatedBondingCurve,
            userATA,
            amountLamports,
            maxSolCost
        );

        console.log(`Buying ${amountSol} SOL of ${mint.toBase58()}...`);
        await executeTransaction(connection, wallet, jito, [ix]);

        // Track Position with High Precision
        let estimatedTokens = BigInt(0);
        try {
            const curveInfo = await connection.getAccountInfo(bondingCurve);
            if (curveInfo) {
                estimatedTokens = BondingCurveManager.getAmountOut(curveInfo.data, amountLamports);
            }
        } catch (e) {
            console.warn("Could not calculate high-precision token amount, using estimate.");
            estimatedTokens = BigInt(Math.floor(Number(amountLamports) / 30));
        }

        PositionManager.getInstance().addPosition(mint, bondingCurve, estimatedTokens, amountSol);

    } catch (e) {
        console.error("Buy failed:", e);
    }
}

async function handleSell(connection: Connection, wallet: Keypair, jito: JitoClient) {
    const mintStr = await new Promise<string>(resolve => rl.question('Token Mint Address to Sell: ', resolve));
    const percentagetStr = await new Promise<string>(resolve => rl.question('Percentage to sell (e.g., 100 for all): ', resolve));

    try {
        const mint = new PublicKey(mintStr);
        const percentage = parseFloat(percentagetStr);

        // Fetch specific amount from balance
        const { userATA } = getDerivedAccounts(mint, wallet.publicKey);
        const balance = await connection.getTokenAccountBalance(userATA);
        if (!balance.value.uiAmount) {
            console.log("No balance found.");
            return;
        }
        const amountToSell = BigInt(Math.floor(Number(balance.value.amount) * (percentage / 100)));

        await executeSell(connection, wallet, jito, mint, amountToSell);

    } catch (e) {
        console.error("Invalid input or error:", e);
    }
}

async function executeSell(connection: Connection, wallet: Keypair, jito: JitoClient, mint: PublicKey, amount: bigint) {
    try {
        console.log(`Selling ${amount.toString()} units of ${mint.toBase58()}...`);
        const { bondingCurve, associatedBondingCurve, userATA } = getDerivedAccounts(mint, wallet.publicKey);

        const ix = PumpFunStrategy.createSellInstruction(
            wallet.publicKey,
            mint,
            bondingCurve,
            associatedBondingCurve,
            userATA,
            amount,
            BigInt(0) // Min SOL output
        );

        await executeTransaction(connection, wallet, jito, [ix]);
    } catch (e) {
        console.error("Sell failed:", e);
    }
}

async function executeTransaction(connection: Connection, wallet: Keypair, jito: JitoClient, instructions: TransactionInstruction[]) {
    try {
        const useJito = process.env.JITO_ENABLED === 'true';

        console.log("Fetching Dynamic Priority Fees...");
        const priorityIxs = await PriorityFeeManager.getDynamicPriorityInstructions(connection);

        // Prepend priority instructions (CU Limit + Price)
        instructions.unshift(...priorityIxs);

        // 1. Build the Versioned Transaction for Simulation
        console.log("Building V0 Transaction for Simulation...");
        const txForSim = await TransactionBuilder.buildV0Transaction(connection, wallet, instructions);

        // 2. Simulate (Safety Check)
        console.log("Simulating...");
        const sim = await connection.simulateTransaction(txForSim);
        if (sim.value.err) {
            console.error("Simulation Failed:", sim.value.err);
            console.error("Logs:", sim.value.logs);
            return;
        }
        console.log("Simulation Successful.");

        // 3. Dispatch to Sender
        const useHelius = process.env.HELIUS_SENDER_ENABLED === 'true';
        const useJito = process.env.JITO_ENABLED === 'true';

        if (useHelius) {
            console.log("Using Helius Smart Sender...");
            const helius = HeliusSender.getInstance();

            // Helius Smart Sender expects instructions. 
            // We pass the instructions which ALREADY contain priority fees from line 272.
            // HeliusSender should just wrap them with a tip and send.
            const sig = await helius.sendSmartTransaction(connection, wallet, instructions);
            if (sig) {
                console.log("Waiting for confirmation...");
                const confirm = await connection.confirmTransaction(sig, 'confirmed');
                if (confirm.value.err) {
                    console.error("Confirmation Error:", confirm.value.err);
                } else {
                    console.log("Confirmed via Helius!");
                }
                return;
            }
        }

        if (useJito) {
            // Default Tip: 0.001 SOL
            const tipLamports = BigInt(1000000);
            const tipIx = jito.createTipInstruction(wallet.publicKey, tipLamports);
            instructions.push(tipIx);

            // Rebuild transaction with tip for Jito
            console.log("Building V0 Transaction for Jito Bundle...");
            const txForJito = await TransactionBuilder.buildV0Transaction(connection, wallet, instructions);

            // Send Bundle (Jito)
            console.log("Sending to Jito Block Engine...");
            const bundleId = await jito.sendBundle([txForJito]);
            console.log(`>> JITO BUNDLE DISPATCHED. Bundle ID: ${bundleId}`);
        } else {
            // Send Standard
            console.log("Sending Standard Transaction...");
            const sig = await connection.sendTransaction(txForSim);
            console.log(`Tx sent: ${sig}`);
            console.log("Waiting for confirmation...");
            await connection.confirmTransaction(sig, 'confirmed');
            console.log("Confirmed.");
        }

    } catch (e) {
        console.error("Execution failed:", e);
    }
}

main().catch(console.error);
