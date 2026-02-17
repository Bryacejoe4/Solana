import { Connection, PublicKey } from '@solana/web3.js';
import { ConnectionManager } from './connection';
import { BondingCurveManager } from './bondingCurve';

interface Position {
    mint: PublicKey;
    bondingCurve: PublicKey;
    amount: bigint;
    entryPrice: number;
    entryTime: number;
}

export class PositionManager {
    private connection: Connection;
    private static instance: PositionManager;
    private positions: Map<string, Position> = new Map();

    private readonly TAKE_PROFIT_PERCENT = 0.50;
    private readonly STOP_LOSS_PERCENT = 0.15;

    private constructor() {
        this.connection = ConnectionManager.getInstance().getConnection();
    }

    public static getInstance(): PositionManager {
        if (!PositionManager.instance) {
            PositionManager.instance = new PositionManager();
        }
        return PositionManager.instance;
    }

    public addPosition(mint: PublicKey, bondingCurve: PublicKey, amount: bigint, costSol: number) {
        const entryPrice = costSol / Number(amount);

        this.positions.set(mint.toBase58(), {
            mint,
            bondingCurve,
            amount,
            entryPrice,
            entryTime: Date.now()
        });
        console.log(`[Position] Tracked ${mint.toBase58()}. Entry: ${entryPrice.toFixed(9)} SOL/unit`);
    }

    public async monitorPositions(sellCallback: (mint: PublicKey, amount: bigint) => Promise<void>) {
        console.log(`[Position] Monitoring ${this.positions.size} active positions...`);

        for (const [mintStr, pos] of this.positions.entries()) {
            try {
                const currentPrice = await this.getPrice(pos.bondingCurve);
                const pnlPercent = (currentPrice - pos.entryPrice) / pos.entryPrice;

                if (pnlPercent >= this.TAKE_PROFIT_PERCENT) {
                    console.log(`[Position] TP Triggered for ${mintStr}! (+${(pnlPercent * 100).toFixed(2)}%)`);
                    await sellCallback(pos.mint, pos.amount);
                    this.positions.delete(mintStr);
                } else if (pnlPercent <= -this.STOP_LOSS_PERCENT) {
                    console.log(`[Position] SL Triggered for ${mintStr}! (${(pnlPercent * 100).toFixed(2)}%)`);
                    await sellCallback(pos.mint, pos.amount);
                    this.positions.delete(mintStr);
                }

            } catch (e) {
                console.error(`[Position] Error monitoring ${mintStr}:`, e);
            }
        }
    }

    private async getPrice(bondingCurve: PublicKey): Promise<number> {
        return await BondingCurveManager.fetchPrice(this.connection, bondingCurve);
    }
}
