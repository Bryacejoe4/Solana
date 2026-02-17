import {
    Connection,
    Keypair,
    TransactionMessage,
    VersionedTransaction,
    AddressLookupTableAccount,
    TransactionInstruction
} from '@solana/web3.js';

export class TransactionBuilder {

    public static async buildV0Transaction(
        connection: Connection,
        payer: Keypair,
        instructions: TransactionInstruction[],
        lookupTables: AddressLookupTableAccount[] = []
    ): Promise<VersionedTransaction> {
        const { blockhash } = await connection.getLatestBlockhash();

        const messageV0 = new TransactionMessage({
            payerKey: payer.publicKey,
            recentBlockhash: blockhash,
            instructions,
        }).compileToV0Message(lookupTables);

        const transaction = new VersionedTransaction(messageV0);
        transaction.sign([payer]);

        return transaction;
    }

    public static async createAndSignBundle(
        connection: Connection,
        payer: Keypair,
        instructionsList: TransactionInstruction[][]
    ): Promise<VersionedTransaction[]> {
        const { blockhash } = await connection.getLatestBlockhash();

        return instructionsList.map(instructions => {
            const messageV0 = new TransactionMessage({
                payerKey: payer.publicKey,
                recentBlockhash: blockhash,
                instructions,
            }).compileToV0Message([]);

            const transaction = new VersionedTransaction(messageV0);
            transaction.sign([payer]);
            return transaction;
        });
    }
}
