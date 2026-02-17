import Client, {
    CommitmentLevel,
    SubscribeRequest,
} from "@triton-one/yellowstone-grpc";
import bs58 from 'bs58';

export class GrpcClient {
    private client: Client;
    private endpoint: string;
    private token: string;
    private isRunning: boolean = false;

    constructor(endpoint: string, token: string) {
        this.endpoint = endpoint;
        this.token = token;
        // @ts-ignore
        this.client = new Client(this.endpoint, this.token, undefined);
    }

    public async start(callback: (signature: string, logs: string[]) => void) {
        if (this.isRunning) return;
        this.isRunning = true;

        console.log(`[gRPC] Connecting to YellowStone gRPC...`);

        try {
            const stream = await this.client.subscribe();

            const PUMP_FUN_PROGRAM = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P";

            const request: SubscribeRequest = {
                accounts: {},
                slots: {},
                transactions: {
                    pumpFun: {
                        vote: false,
                        failed: false,
                        signature: undefined,
                        accountInclude: [PUMP_FUN_PROGRAM],
                        accountExclude: [],
                        accountRequired: [],
                    },
                },
                blocks: {},
                blocksMeta: {},
                entry: {},
                commitment: CommitmentLevel.PROCESSED,
                accountsDataSlice: [],
                transactionsStatus: {},
                ping: undefined,
            };

            stream.write(request);

            console.log("[gRPC] Stream established. Waiting for events...");

            stream.on("data", (data: any) => {
                if (data.transaction && data.transaction.transaction) {
                    const tx = data.transaction.transaction;
                    const signature = bs58.encode(tx.signature);
                    const logMessages = tx.meta?.logMessages || [];

                    const isCreation = logMessages.some((log: string) => log.includes("Instruction: Create"));
                    if (isCreation) {
                        callback(signature, logMessages);
                    }
                }
            });

            stream.on("error", (error: any) => {
                console.error("[gRPC] Stream Error:", error);
                this.isRunning = false;
            });

            stream.on("end", () => {
                console.log("[gRPC] Stream Ended");
                this.isRunning = false;
            });

        } catch (e) {
            console.error("[gRPC] Connection Failed. Check your Endpoint/Token.", e);
            this.isRunning = false;
        }
    }
}
