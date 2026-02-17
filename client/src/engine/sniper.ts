import { Connection, PublicKey } from '@solana/web3.js';
import { ConnectionManager } from './connection';

export class Sniper {
    private connection: Connection;
    private isRunning: boolean = false;
    private readonly PUMP_FUN_PROGRAM_ID = new PublicKey("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");

    constructor() {
        this.connection = ConnectionManager.getInstance().getConnection();
    }

    public async startListener(callback: (signature: string, logs: string[]) => void) {
        if (this.isRunning) return;
        this.isRunning = true;
        console.log("Starting Sniper Listener for Pump.fun...");

        this.connection.onLogs(
            this.PUMP_FUN_PROGRAM_ID,
            async (logs, ctx) => {
                if (logs.err) return;

                const isCreation = logs.logs.some(log => log.includes("Instruction: Create"));

                if (isCreation) {
                    callback(logs.signature, logs.logs);
                }
            },
            "processed"
        );
    }

    public stop() {
        this.isRunning = false;
    }
}
