import {
    PublicKey,
    TransactionInstruction,
    SystemProgram,
    SYSVAR_RENT_PUBKEY
} from '@solana/web3.js';

export const PUMP_FUN_PROGRAM_ID = new PublicKey("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");

export class PumpFunStrategy {

    public static createBuyInstruction(
        buyer: PublicKey,
        mint: PublicKey,
        bondingCurve: PublicKey,
        associatedBondingCurve: PublicKey,
        associatedUserAccount: PublicKey,
        amount: bigint,
        maxSolCost: bigint
    ): TransactionInstruction {

        const discriminator = Buffer.from([0x66, 0x06, 0x3d, 0x12, 0x01, 0xda, 0xeb, 0xea]);
        const data = Buffer.alloc(8 + 8 + 8);
        discriminator.copy(data, 0);
        data.writeBigUInt64LE(amount, 8);
        data.writeBigUInt64LE(maxSolCost, 16);

        return new TransactionInstruction({
            programId: PUMP_FUN_PROGRAM_ID,
            keys: [
                { pubkey: new PublicKey("4wTV1YmiEkRvAtNtsSGPtUrqefndJP1KAk8bJqDRxryf"), isSigner: false, isWritable: false }, // global
                { pubkey: new PublicKey("CebN5WGQ4jvEPvsVU4EoHEPGzq1VV7AbicfcvWiyZn4U"), isSigner: false, isWritable: true }, // fee recipient
                { pubkey: mint, isSigner: false, isWritable: false },
                { pubkey: bondingCurve, isSigner: false, isWritable: true },
                { pubkey: associatedBondingCurve, isSigner: false, isWritable: true },
                { pubkey: associatedUserAccount, isSigner: false, isWritable: true },
                { pubkey: buyer, isSigner: true, isWritable: true },
                { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
                { pubkey: new PublicKey("CcVNgwKGFF4B7qAg4pXYQ2p1P9y4XjQ0pJb00000000"), isSigner: false, isWritable: false }, // token program
                { pubkey: SYSVAR_RENT_PUBKEY, isSigner: false, isWritable: false },
                { pubkey: new PublicKey("Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjj46Pu5y4382"), isSigner: false, isWritable: false }, // event authority
                { pubkey: PUMP_FUN_PROGRAM_ID, isSigner: false, isWritable: false }, // program
            ],
            data: data
        });
    }

    public static createSellInstruction(
        seller: PublicKey,
        mint: PublicKey,
        bondingCurve: PublicKey,
        associatedBondingCurve: PublicKey,
        associatedUserAccount: PublicKey,
        amount: bigint,
        minSolOutput: bigint
    ): TransactionInstruction {
        const discriminator = Buffer.from([0x33, 0xe6, 0x85, 0xa4, 0x01, 0x7f, 0x83, 0xad]);
        const data = Buffer.alloc(8 + 8 + 8);
        discriminator.copy(data, 0);
        data.writeBigUInt64LE(amount, 8);
        data.writeBigUInt64LE(minSolOutput, 16);

        return new TransactionInstruction({
            programId: PUMP_FUN_PROGRAM_ID,
            keys: [
                { pubkey: new PublicKey("4wTV1YmiEkRvAtNtsSGPtUrqefndJP1KAk8bJqDRxryf"), isSigner: false, isWritable: false }, // global
                { pubkey: new PublicKey("CebN5WGQ4jvEPvsVU4EoHEPGzq1VV7AbicfcvWiyZn4U"), isSigner: false, isWritable: true }, // fee recipient
                { pubkey: mint, isSigner: false, isWritable: false },
                { pubkey: bondingCurve, isSigner: false, isWritable: true },
                { pubkey: associatedBondingCurve, isSigner: false, isWritable: true },
                { pubkey: associatedUserAccount, isSigner: false, isWritable: true },
                { pubkey: seller, isSigner: true, isWritable: true },
                { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
                { pubkey: new PublicKey("CcVNgwKGFF4B7qAg4pXYQ2p1P9y4XjQ0pJb00000000"), isSigner: false, isWritable: false }, // token program
                { pubkey: new PublicKey("Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjj46Pu5y4382"), isSigner: false, isWritable: false }, // event authority
                { pubkey: PUMP_FUN_PROGRAM_ID, isSigner: false, isWritable: false }, // program
            ],
            data: data
        });
    }
}
