import { Keypair } from '@solana/web3.js';
import * as fs from 'fs';
import bs58 from 'bs58';

export function loadKeypair(source: string): Keypair {
    try {
        const bytes = JSON.parse(source);
        if (Array.isArray(bytes)) {
            return Keypair.fromSecretKey(Uint8Array.from(bytes));
        }
    } catch (e) { }

    try {
        const bytes = bs58.decode(source);
        return Keypair.fromSecretKey(bytes);
    } catch (e) { }

    if (fs.existsSync(source)) {
        try {
            const fileContent = fs.readFileSync(source, 'utf-8');
            const bytes = JSON.parse(fileContent);
            return Keypair.fromSecretKey(Uint8Array.from(bytes));
        } catch (e) {
            throw new Error(`Failed to load keypair from file: ${source}`);
        }
    }

    throw new Error(`Could not load keypair from source: ${source}`);
}

export function loadKeypairFromEnv(envVar: string): Keypair {
    const val = process.env[envVar];
    if (!val) {
        throw new Error(`Environment variable ${envVar} is not set`);
    }
    return loadKeypair(val);
}
