import { Connection } from '@solana/web3.js';
import * as dotenv from 'dotenv';

dotenv.config();

export class ConnectionManager {
    private static instance: ConnectionManager;
    public connection: Connection;
    public rpcUrl: string;

    private constructor() {
        this.rpcUrl = process.env.RPC_ENDPOINT || 'https://api.mainnet-beta.solana.com';
        this.connection = new Connection(this.rpcUrl, 'confirmed');
    }

    public static getInstance(): ConnectionManager {
        if (!ConnectionManager.instance) {
            ConnectionManager.instance = new ConnectionManager();
        }
        return ConnectionManager.instance;
    }

    public getConnection(): Connection {
        return this.connection;
    }
}
