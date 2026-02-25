import { Connection, PublicKey } from "@solana/web3.js";
import bs58 from "bs58";

const connection = new Connection("https://api.mainnet-beta.solana.com");
const pumpProgramId = new PublicKey("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");

async function main() {
    try {
        console.log("Fetching recent signatures...");
        const sigs = await connection.getSignaturesForAddress(pumpProgramId, { limit: 10 });
        for (const sigInfo of sigs) {
            console.log(`Checking tx: ${sigInfo.signature}`);
            const tx = await connection.getTransaction(sigInfo.signature, { maxSupportedTransactionVersion: 0 });
            if (!tx || !tx.transaction || !tx.transaction.message) continue;

            const message = tx.transaction.message;
            let accountKeys: PublicKey[] = [];
            // Handle both legacy and v0 messages
            if ('staticAccountKeys' in message) {
                // v0 message
                accountKeys = message.staticAccountKeys;
            } else {
                // legacy message
                accountKeys = (message as any).accountKeys;
            }

            for (const ix of message.compiledInstructions) {
                const programId = accountKeys[ix.programIdIndex];
                if (programId && programId.toBase58() === pumpProgramId.toBase58()) {
                    let data;
                    if (ix.data instanceof Uint8Array) {
                        data = ix.data;
                    } else if (typeof ix.data === "string") {
                        data = bs58.decode(ix.data);
                    } else {
                        data = Buffer.from(ix.data as unknown as number[]);
                    }

                    // pump.fun buy discriminator is: 66 06 3d 12 01 da eb ea
                    const buyDisc = Buffer.from([0x66, 0x06, 0x3d, 0x12, 0x01, 0xda, 0xeb, 0xea]);
                    if (data.length >= 8 && Buffer.from(data.slice(0, 8)).equals(buyDisc)) {
                        console.log("FOUND BUY INSTRUCTION! Accounts:");

                        ix.accountKeyIndexes.forEach((accIdx, idx) => {
                            let pubkey;
                            if (accIdx < accountKeys.length) pubkey = accountKeys[accIdx];
                            else pubkey = "ALT Account " + accIdx;
                            console.log(`[${idx}]: ${pubkey ? pubkey.toString() : 'unknown'}`);
                        });
                        return; // Found and logged, we're done
                    }
                }
            }
        }
    } catch (e) {
        console.error(e);
    }
}

main();
