import {
    PublicKey,
    TransactionInstruction,
    SystemProgram,
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
        maxSolCost: bigint,
        creator: PublicKey = SystemProgram.programId
    ): TransactionInstruction {
        const discriminator = Buffer.from([0x66, 0x06, 0x3d, 0x12, 0x01, 0xda, 0xeb, 0xea]);
        const data = Buffer.alloc(8 + 8 + 8);
        discriminator.copy(data, 0);
        data.writeBigUInt64LE(amount, 8);
        data.writeBigUInt64LE(maxSolCost, 16);

        const [creatorVault] = PublicKey.findProgramAddressSync(
            [Buffer.from("creator-vault"), creator.toBuffer()],
            PUMP_FUN_PROGRAM_ID
        );
        const [globalVolumeAccumulator] = PublicKey.findProgramAddressSync(
            [Buffer.from("global_volume_accumulator")],
            PUMP_FUN_PROGRAM_ID
        );
        const [userVolumeAccumulator] = PublicKey.findProgramAddressSync(
            [Buffer.from("user_volume_accumulator"), buyer.toBuffer()],
            PUMP_FUN_PROGRAM_ID
        );

        return new TransactionInstruction({
            programId: PUMP_FUN_PROGRAM_ID,
            keys: [
                { pubkey: new PublicKey("4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf"), isSigner: false, isWritable: false }, // global
                { pubkey: new PublicKey("62qc2CNXwrYqQScmEdiZFFAnJR262PxWEuNQtxfafNgV"), isSigner: false, isWritable: true }, // fee recipient
                { pubkey: mint, isSigner: false, isWritable: false }, // mint
                { pubkey: bondingCurve, isSigner: false, isWritable: true }, // bondingCurve
                { pubkey: associatedBondingCurve, isSigner: false, isWritable: true }, // associatedBondingCurve
                { pubkey: associatedUserAccount, isSigner: false, isWritable: true }, // associatedUser
                { pubkey: buyer, isSigner: true, isWritable: true }, // user
                { pubkey: SystemProgram.programId, isSigner: false, isWritable: false }, // systemProgram
                { pubkey: new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"), isSigner: false, isWritable: false }, // token program
                { pubkey: new PublicKey("SysvarRent111111111111111111111111111111111"), isSigner: false, isWritable: false }, // rent
                { pubkey: new PublicKey("Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1"), isSigner: false, isWritable: false }, // event authority
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
        minSolOutput: bigint,
        creator: PublicKey = SystemProgram.programId
    ): TransactionInstruction {
        const discriminator = Buffer.from([0x33, 0xe6, 0x85, 0xa4, 0x01, 0x7f, 0x83, 0xad]);
        const data = Buffer.alloc(8 + 8 + 8);
        discriminator.copy(data, 0);
        data.writeBigUInt64LE(amount, 8);
        data.writeBigUInt64LE(minSolOutput, 16);

        const [creatorVault] = PublicKey.findProgramAddressSync(
            [Buffer.from("creator-vault"), creator.toBuffer()],
            PUMP_FUN_PROGRAM_ID
        );
        const [globalVolumeAccumulator] = PublicKey.findProgramAddressSync(
            [Buffer.from("global_volume_accumulator")],
            PUMP_FUN_PROGRAM_ID
        );
        const [userVolumeAccumulator] = PublicKey.findProgramAddressSync(
            [Buffer.from("user_volume_accumulator"), seller.toBuffer()],
            PUMP_FUN_PROGRAM_ID
        );

        return new TransactionInstruction({
            programId: PUMP_FUN_PROGRAM_ID,
            keys: [
                { pubkey: new PublicKey("4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf"), isSigner: false, isWritable: false }, // global
                { pubkey: new PublicKey("62qc2CNXwrYqQScmEdiZFFAnJR262PxWEuNQtxfafNgV"), isSigner: false, isWritable: true }, // fee recipient
                { pubkey: mint, isSigner: false, isWritable: false }, // mint
                { pubkey: bondingCurve, isSigner: false, isWritable: true }, // bondingCurve
                { pubkey: associatedBondingCurve, isSigner: false, isWritable: true }, // associatedBondingCurve
                { pubkey: associatedUserAccount, isSigner: false, isWritable: true }, // associatedUser
                { pubkey: seller, isSigner: true, isWritable: true }, // seller
                { pubkey: SystemProgram.programId, isSigner: false, isWritable: false }, // systemProgram
                { pubkey: new PublicKey("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA"), isSigner: false, isWritable: false }, // token program
                { pubkey: new PublicKey("SysvarRent111111111111111111111111111111111"), isSigner: false, isWritable: false }, // rent
                { pubkey: new PublicKey("Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1"), isSigner: false, isWritable: false }, // event authority
                { pubkey: PUMP_FUN_PROGRAM_ID, isSigner: false, isWritable: false }, // program
            ],
            data: data
        });
    }
}
