const https = require('https');
const { Transaction } = require('@solana/web3.js');

const data = JSON.stringify({
    "action": "buy",
    "mint": "DRLNhjM7jusYFPF1qade1dBD1qhgds7oAfdKs51Vpump",
    "denominatedInSol": "true",
    "amount": 0.01,
    "slippage": 10,
    "priorityFee": 0.00001,
    "pool": "pump"
});

const req = https.request('https://pumpportal.fun/api/trade-local', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' }
}, (res) => {
    let body = '';
    res.on('data', chunk => body += chunk);
    res.on('end', () => {
        try {
            const txBytes = Buffer.from(body, 'base64');
            const tx = Transaction.from(txBytes);

            console.log("ACCOUNTS IN TRANSACTION:");
            tx.compileMessage().accountKeys.forEach((k, i) => console.log(i + ": " + k.toBase58()));

            console.log("\nINSTRUCTIONS:");
            tx.instructions.forEach(ix => {
                if (ix.programId.toBase58() === "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P") {
                    console.log("PUMP.FUN IX ACCOUNTS ORDER:");
                    ix.keys.forEach((k, i) => {
                        console.log(`Arg ${i}: ${k.pubkey.toBase58()} (isSigner: ${k.isSigner}, isWritable: ${k.isWritable})`);
                    });
                }
            });
        } catch (e) {
            console.error("Error decoding:", e.message);
            console.log("Raw response:", body);
        }
    });
});

req.on('error', e => console.error(e));
req.write(data);
req.end();
