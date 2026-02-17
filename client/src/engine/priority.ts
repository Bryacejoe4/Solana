import { Connection, ComputeBudgetProgram, TransactionInstruction, PublicKey } from '@solana/web3.js';

const PUMP_PROGRAM = new PublicKey("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");

export class PriorityFeeManager {
    static async getDynamicPriorityInstructions(connection: Connection): Promise<TransactionInstruction[]> {
        const ixs: TransactionInstruction[] = [];

        ixs.push(ComputeBudgetProgram.setComputeUnitLimit({ units: 100_000 }));

        try {
            const recentFees = await connection.getRecentPrioritizationFees({
                lockedWritableAccounts: [PUMP_PROGRAM]
            });

            let maxFee = 0;
            if (recentFees.length > 0) {
                const fees = recentFees.map(f => f.prioritizationFee);
                maxFee = Math.max(...fees);
            }

            const feeToSet = maxFee > 0 ? Math.ceil(maxFee * 1.2) : 5_000;

            console.log(`[Priority] Setting Fee: ${feeToSet} microLamports (Limit: 100k)`);

            ixs.push(ComputeBudgetProgram.setComputeUnitPrice({ microLamports: feeToSet }));

        } catch (e) {
            console.error("Error fetching priority fees, using default high fee:", e);
            ixs.push(ComputeBudgetProgram.setComputeUnitPrice({ microLamports: 10_000 }));
        }

        return ixs;
    }
}
