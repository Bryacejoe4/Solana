import { Connection, Keypair, VersionedTransaction, TransactionMessage, TransactionInstruction, ComputeBudgetProgram, SystemProgram, PublicKey } from '@solana/web3.js';
import { TransactionBuilder } from './transaction';
import axios from 'axios';
import * as dotenv from 'dotenv';
import { loadKeypairFromEnv } from '../utils/keys';

dotenv.config();

export class HeliusSender {
    private static instance: HeliusSender;
    private apiKey: string;
    private rpcUrl: string;

    // Designated Tip Accounts for Jito/Helius Smart Sender
    // Source: Helius Docs
    private static TIP_ACCOUNTS = [
        "96gYZGLnJYVFmbjzopPSU6QiEV5fGqZNyN9nmNhvrZU5",
        "HfX7HK6t3DKtDQcSaBRBS7WmMf57Xru6fW9tMAZfd8qU",
        "Cw8CFyM9FkoMi7K7Crf6HNQqf4uEMzpKw6QNghXLvLkY",
        "ADaUMid9yfUytqMBgopXSjbCpNSRgB5sZ1bcS9imeZEu",
        "DfXygSm4jCyNCybVYYK6DwvWqjKkf8tX74eb5nrbExi",
        "ADuUkR4ykGytmnb5LHcUyU75zgDoE8WuyxwZjNXkPPNq",
        "DttWaMuVvTiduZRnguLF7jNxTgiMBZ1hyAumKUiL2KRL",
        "3AVi9Tg9Uo68tJfuvoKvqKNWKkC5wPdSSdeBnIzKZ6jJ"
    ];

    private constructor() {
        this.apiKey = process.env.HELIUS_API_KEY || "";
        if (!this.apiKey) {
            console.warn("WARNING: HELIUS_API_KEY is missing via .env. Smart Sender will fail.");
        }
        this.rpcUrl = `https://mainnet.helius-rpc.com/?api-key=${this.apiKey}`;
    }

    public static getInstance(): HeliusSender {
        if (!HeliusSender.instance) {
            HeliusSender.instance = new HeliusSender();
        }
        return HeliusSender.instance;
    }

    public getRandomTipAccount(): PublicKey {
        const index = Math.floor(Math.random() * HeliusSender.TIP_ACCOUNTS.length);
        return new PublicKey(HeliusSender.TIP_ACCOUNTS[index]);
    }

    public async sendSmartTransaction(
        connection: Connection,
        wallet: Keypair,
        instructions: TransactionInstruction[],
        priorityFeeMicroLamports: number = 100000,
        tipAmountLamports: bigint = BigInt(1000000) // Default 0.001 SOL
    ): Promise<string | null> {
        try {
            // 2. Add Jito/Helius Tip
            const tipAccount = this.getRandomTipAccount();
            const tipIx = SystemProgram.transfer({
                fromPubkey: wallet.publicKey,
                toPubkey: tipAccount,
                lamports: Number(tipAmountLamports)
            });

            const allInstructions = [
                ...instructions,
                tipIx
            ];

            // 3. Build Versioned Transaction
            const { value: { blockhash } } = await connection.getLatestBlockhashAndContext('confirmed');

            const messageV0 = new TransactionMessage({
                payerKey: wallet.publicKey,
                recentBlockhash: blockhash,
                instructions: allInstructions,
            }).compileToV0Message();

            const transaction = new VersionedTransaction(messageV0);
            transaction.sign([wallet]);

            const serializedTx = Buffer.from(transaction.serialize()).toString('base64');

            // 4. Send via Helius Smart Sender Endpoint
            const smartSenderEndpoint = "https://sender.helius-rpc.com/fast";

            console.log(`Sending via Helius Smart Sender...`);

            const response = await axios.post(smartSenderEndpoint, {
                jsonrpc: '2.0',
                id: 'helius-sender',
                method: 'sendTransaction',
                params: [
                    serializedTx,
                    {
                        encoding: 'base64',
                        skipPreflight: true, // MANDATORY for Smart Sender
                        maxRetries: 0       // MANDATORY for Smart Sender
                    }
                ]
            }, {
                headers: { 'Content-Type': 'application/json' }
            });

            if (response.data.error) {
                console.error("Helius Smart Sender Error:", response.data.error);
                return null;
            }

            const signature = response.data.result;
            console.log(`Helius Smart Sender Sent: ${signature}`);
            return signature;

        } catch (e: any) {
            console.error("Failed to send via Helius Smart Sender:", e.message || e);
            return null;
        }
    }
}
