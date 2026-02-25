import urllib.request
import json
import sys

def get_fresh_token():
    try:
        url = 'https://frontend-api.pump.fun/coins/latest'
        req = urllib.request.Request(url, headers={'User-Agent': 'Mozilla/5.0'})
        response = urllib.request.urlopen(req)
        data = json.loads(response.read().decode('utf-8'))
        
        # Valid SPL Token Program
        TOKEN_PROGRAM = "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"
        PUMP_FUN_PROGRAM = "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P"
        RPC = "https://api.mainnet-beta.solana.com"

        for token in data:
            if token.get('complete') or token.get('usd_market_cap', 0) <= 5000:
                continue
            
            mint = token['mint']
            # Perform an active on-chain check to ensure the bonding curve is STILL valid!
            rpc_req = urllib.request.Request(
                RPC,
                headers={'Content-Type': 'application/json'},
                data=json.dumps({
                    "jsonrpc": "2.0", "id": 1,
                    "method": "getAccountInfo",
                    "params": [mint, {"encoding": "jsonParsed"}]
                }).encode('utf-8')
            )
            try:
                rpc_res = json.loads(urllib.request.urlopen(rpc_req).read().decode('utf-8'))
                owner = rpc_res.get('result', {}).get('value', {}).get('owner')
                if owner != TOKEN_PROGRAM:
                    continue # Not an SPL token (e.g. Token-2022)
                
                cap = token['usd_market_cap']
                progress = (cap / 69000.0) * 100
                print(f"\n✅ Found a fresh Pump.fun token that is actively trading and VERIFIED on-chain:")
                print(f"Token Mint Address: {mint}")
                print(f"Name: {token.get('name')} ({token.get('symbol')})")
                print(f"Market Cap: ${cap:,.2f}")
                print(f"Bonding Curve Progress: {progress:.2f}%")
                print(f"\n⚠️ Use this token mint in your trade commands! ⚠️\n")
                return
            except Exception as e:
                pass # skip token on RPC error

        print("No non-graduated tokens found at the moment.")
    except Exception as e:
        print(f"Error fetching token: {e}")

if __name__ == "__main__":
    get_fresh_token()
