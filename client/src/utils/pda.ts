import { PublicKey } from '@solana/web3.js';

export const TOKEN_PROGRAM_ID = new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
export const ASSOCIATED_TOKEN_PROGRAM_ID = new PublicKey("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
export const PUMP_FUN_PROGRAM = new PublicKey("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");

export function getDerivedAccounts(mint: PublicKey, owner: PublicKey) {
    const [bondingCurve] = PublicKey.findProgramAddressSync(
        [Buffer.from("bonding-curve"), mint.toBuffer()],
        PUMP_FUN_PROGRAM
    );

    const [associatedBondingCurve] = PublicKey.findProgramAddressSync(
        [bondingCurve.toBuffer(), TOKEN_PROGRAM_ID.toBuffer(), mint.toBuffer()],
        ASSOCIATED_TOKEN_PROGRAM_ID
    );

    const [userATA] = PublicKey.findProgramAddressSync(
        [owner.toBuffer(), TOKEN_PROGRAM_ID.toBuffer(), mint.toBuffer()],
        ASSOCIATED_TOKEN_PROGRAM_ID
    );

    return {
        bondingCurve,
        associatedBondingCurve,
        userATA
    };
}
