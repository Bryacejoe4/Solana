import { Connection, VersionedTransaction, SimulateTransactionConfig } from '@solana/web3.js';

export class SimulationEngine {
    private connection: Connection;

    constructor(connection: Connection) {
        this.connection = connection;
    }

    public async simulate(
        transaction: VersionedTransaction,
        config: SimulateTransactionConfig = { sigVerify: false, replaceRecentBlockhash: true }
    ): Promise<any> {
        const { value } = await this.connection.simulateTransaction(transaction, config);

        if (value.err) {
            console.error("Simulation Error:", value.err);
            console.error("Logs:", value.logs);
            throw new Error(`Simulation failed: ${JSON.stringify(value.err)}`);
        }

        return value;
    }
}
