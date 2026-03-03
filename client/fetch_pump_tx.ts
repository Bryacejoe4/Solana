import { Connection, PublicKey } from '@solana/web3.js';
import bs58 from 'bs58';

async function main() {
    const connection = new Connection("https://api.mainnet-beta.solana.com", "confirmed");
    const programId = new PublicKey("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");
    
    console.log("Fetching signatures for Pump.fun program...");
    const sigs = await connection.getSignaturesForAddress(programId, { limit: 10 });
    
    for (const sig of sigs) {
        if (sig.err) continue;
        const tx = await connection.getTransaction(sig.signature, { maxSupportedTransactionVersion: 0 });
        if (!tx || !tx.meta || tx.meta.err) continue;
        
        let found = false;
        
        // message is a Message for v0
        const accountKeys = tx.transaction.message.staticAccountKeys; // We might need to resolve LUTs but let's stick to easy ones
        if (!accountKeys) {
            continue; // Maybe v0, let's just skip, actually the signature getTransaction returns message.accountKeys 
        }
        
        const allKeys = [...tx.transaction.message.staticAccountKeys];
        // add LUTs if any? no, keep it simple, just log the indices
        
        for (const ix of tx.transaction.message.compiledInstructions) {
            const programKey = allKeys[ix.programIdIndex];
            if (programKey && programKey.toBase58() === programId.toBase58()) {
                if (ix.data.length === 24 && ix.data[0] === 0x66) { // Buy discriminator 0x6606...
                    console.log(`\n\n=== BUY TRANSACTION: ${sig.signature} ===`);
                    ix.accountKeyIndexes.forEach((idx, num) => {
                        console.log(`Account ${num}: ${allKeys[idx]?.toBase58()} (Index ${idx})`);
                    });
                    found = true;
                    return; // exit early
                }
            }
        }
    }
}

main().catch(console.error);
