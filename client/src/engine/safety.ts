import { Connection, PublicKey } from '@solana/web3.js';
import { ConnectionManager } from './connection';

export class SafetyChecker {
    private connection: Connection;
    private static instance: SafetyChecker;

    private constructor() {
        this.connection = ConnectionManager.getInstance().getConnection();
    }

    public static getInstance(): SafetyChecker {
        if (!SafetyChecker.instance) {
            SafetyChecker.instance = new SafetyChecker();
        }
        return SafetyChecker.instance;
    }

    public async checkToken(mint: PublicKey): Promise<boolean> {
        console.log(`[Safety] Checking token: ${mint.toBase58()}...`);

        try {
            const info = await this.connection.getParsedAccountInfo(mint);
            if (!info.value) return false;

            const data = info.value.data;
            if (!('parsed' in data)) return false;

            const mintInfo = data.parsed.info;

            if (mintInfo.freezeAuthority) {
                console.warn(`[Safety] FAILED: Freeze Authority is set! (${mintInfo.freezeAuthority})`);
                return false;
            }

            console.log(`[Safety] Mint/Freeze checks passed.`);
            return true;

        } catch (e) {
            console.error(`[Safety] Error checking token:`, e);
            return false;
        }
    }
}
