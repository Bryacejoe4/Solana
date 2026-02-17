import { Keypair, VersionedTransaction, PublicKey, TransactionInstruction, SystemProgram } from '@solana/web3.js';
import { searcher, bundle } from 'jito-ts';
import * as dotenv from 'dotenv';
import { loadKeypairFromEnv } from '../utils/keys';

dotenv.config();

export class JitoClient {
    private static instance: JitoClient;
    private searcherClient: searcher.SearcherClient | null = null;
    private authKeypair: Keypair | null = null;
    private blockEngineUrl: string;

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
        this.blockEngineUrl = process.env.JITO_BLOCK_ENGINE_URL || 'https://nyc.mainnet.block-engine.jito.wtf';

        if (process.env.JITO_AUTH_PRIVATE_KEY) {
            try {
                this.authKeypair = loadKeypairFromEnv('JITO_AUTH_PRIVATE_KEY');
                console.log("Jito Auth Keypair loaded.");
            } catch (e) {
                console.error("Failed to load Jito Auth Keypair:", e);
            }
        }
    }

    public static getInstance(): JitoClient {
        if (!JitoClient.instance) {
            JitoClient.instance = new JitoClient();
        }
        return JitoClient.instance;
    }

    public getRandomTipAccount(): PublicKey {
        const index = Math.floor(Math.random() * JitoClient.TIP_ACCOUNTS.length);
        return new PublicKey(JitoClient.TIP_ACCOUNTS[index]);
    }

    public createTipInstruction(payer: PublicKey, tipAmountLamports: bigint): TransactionInstruction {
        const tipAccount = this.getRandomTipAccount();
        return SystemProgram.transfer({
            fromPubkey: payer,
            toPubkey: tipAccount,
            lamports: Number(tipAmountLamports)
        });
    }

    public async getSearcherClient(): Promise<searcher.SearcherClient> {
        if (!this.searcherClient) {
            if (!this.authKeypair) {
                throw new Error("Jito Auth Keypair not loaded. Cannot create Searcher Client.");
            }
            this.searcherClient = searcher.searcherClient(
                this.blockEngineUrl,
                this.authKeypair
            );
        }
        return this.searcherClient;
    }

    public async sendBundle(
        transactions: VersionedTransaction[]
    ): Promise<string> {
        const client = await this.getSearcherClient();
        const b = new bundle.Bundle(transactions, 5);

        if (b instanceof Error) {
            throw b;
        }

        try {
            const bundleId = await client.sendBundle(b);
            return bundleId;
        } catch (e) {
            console.error("Error sending bundle:", e);
            throw e;
        }
    }
}
