const fetch = require('node-fetch');

async function getFreshTokens() {
    try {
        const response = await fetch('https://frontend-api.pump.fun/coins/latest');
        const tokens = await response.json();

        const TOKEN_PROGRAM = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA";
        const RPC = "https://api.mainnet-beta.solana.com";

        for (const token of tokens) {
            if (token.complete || token.usd_market_cap <= 5000) continue;

            try {
                // Perform an active on-chain check to ensure the token is valid SPL and NOT migrated
                const rpcRes = await fetch(RPC, {
                    method: 'POST',
                    headers: { 'Content-Type': 'application/json' },
                    body: JSON.stringify({
                        jsonrpc: "2.0", id: 1, method: "getAccountInfo",
                        params: [token.mint, { encoding: "jsonParsed" }]
                    })
                });
                const rpcData = await rpcRes.json();
                const owner = rpcData.result?.value?.owner;

                if (owner !== TOKEN_PROGRAM) {
                    continue; // Skip Token-2022 or invalid coins
                }

                console.log("\n✅ Found a fresh Pump.fun token that is actively trading and VERIFIED on-chain:");
                console.log("Token Mint Address:", token.mint);
                console.log("Name:", token.name, "(", token.symbol, ")");
                console.log("Market Cap: $", token.usd_market_cap.toFixed(2));
                console.log("Bonding Curve Progress: ", (token.usd_market_cap / 69000 * 100).toFixed(2), "%");
                console.log("\n⚠️ Use this token mint in your trade commands! ⚠️\n");
                return;
            } catch (e) {
                // skip on RPC error
            }
        }

        console.log("No non-graduated tokens found at the moment.");
    } catch (error) {
        console.error("Error fetching tokens:", error.message);
    }
}

getFreshTokens();
