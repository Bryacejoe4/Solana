import { Connection, PublicKey } from '@solana/web3.js';
import { Buffer } from 'buffer';

export class BondingCurveManager {
    static getPriceFromData(data: Buffer): number {
        if (data.length < 48) return 0;

        const virtualTokenReserves = data.readBigUInt64LE(8);
        const virtualSolReserves = data.readBigUInt64LE(16);

        if (Number(virtualTokenReserves) === 0) return 0;

        const price = Number(virtualSolReserves) / Number(virtualTokenReserves);
        return price;
    }

    static async fetchPrice(connection: Connection, bondingCurve: PublicKey): Promise<number> {
        try {
            const accountInfo = await connection.getAccountInfo(bondingCurve);
            if (!accountInfo) return 0;
            return this.getPriceFromData(accountInfo.data);
        } catch (e) {
            console.error("Error fetching curve:", e);
            return 0;
        }
    }

    static getAmountOut(data: Buffer, solAmount: bigint): bigint {
        if (data.length < 48) return BigInt(0);

        const virtualTokenReserves = data.readBigUInt64LE(8);
        const virtualSolReserves = data.readBigUInt64LE(16);

        // Standard Pump.fun bonding curve swap formula:
        // (y * x) / (y + dy) = new_x
        // dx = x - new_x
        // where y is SOL reserves, x is token reserves, dy is SOL input.

        const y = virtualSolReserves;
        const x = virtualTokenReserves;
        const dy = solAmount;

        const newX = (y * x) / (y + dy);
        const amountOut = x - newX;

        return amountOut;
    }
}
