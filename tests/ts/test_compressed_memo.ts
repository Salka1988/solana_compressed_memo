import { expect } from "chai";
import { describe, it, before } from "mocha";
import {
    Connection,
    Keypair,
    PublicKey,
    Transaction,
    TransactionInstruction,
    SystemProgram,
    sendAndConfirmTransaction,
} from "@solana/web3.js";
import { serialize, deserialize } from "borsh";

class CreateCompressedMemoLayout {
    enumTag: number;
    memo: string;

    constructor(fields: { enumTag: number; memo: string }) {
        this.enumTag = fields.enumTag;
        this.memo = fields.memo;
    }
}

const InstructionSchema = new Map([
    [
        CreateCompressedMemoLayout,
        {
            kind: "struct",
            fields: [
                ["enumTag", "u8"],
                ["memo", "string"],
            ],
        },
    ],
]);

describe("CreateCompressedMemo Tests", () => {
    let programId: PublicKey;

    before(() => {
        // Parse `--program-id=<PUBKEY>` from process.argv
        const args = process.argv.slice(2);
        const programIdArg = args.find((arg) => arg.startsWith("--program-id="));
        if (!programIdArg) {
            throw new Error("Missing required --program-id argument. Use: --program-id=<PROGRAM_ID>");
        }

        const programIdStr = programIdArg.split("=")[1];
        programId = new PublicKey(programIdStr);
        console.log("Using Program ID:", programId.toBase58());
    });

    it("should send and verify a compressed memo transaction", async () => {
        const connection = new Connection("http://127.0.0.1:8899", "confirmed");

        const payer = Keypair.generate();
        const airdropSig = await connection.requestAirdrop(payer.publicKey, 1e9); // 1 SOL
        await connection.confirmTransaction(airdropSig);
        console.log("Payer created and funded:", payer.publicKey.toBase58());

        const newAccount = Keypair.generate();

        // Build Instruction Data (Borsh)
        const memoStr = "Hello from TypeScript test!";
        const instructionData = new CreateCompressedMemoLayout({
            enumTag: 1,
            memo: memoStr,
        });
        const serializedData = serialize(InstructionSchema, instructionData);

        // Construct the Transaction Instruction
        const ix = new TransactionInstruction({
            programId,
            keys: [
                { pubkey: payer.publicKey, isSigner: true, isWritable: true },
                { pubkey: newAccount.publicKey, isSigner: true, isWritable: true },
                { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
            ],
            data: Buffer.from(serializedData),
        });

        // Send single Transaction with the instruction
        const tx = new Transaction().add(ix);
        const txSig = await sendAndConfirmTransaction(connection, tx, [payer, newAccount]);
        console.log("Transaction Signature:", txSig);

        // Verify the Data in newAccount
        const newAcctInfo = await connection.getAccountInfo(newAccount.publicKey);
        expect(newAcctInfo).to.not.be.null;
        if (!newAcctInfo) {
            return; // TS guard
        }

        expect(newAcctInfo.data.length).to.equal(32);

        // Deserialize the memo for verification
        const deserializedInstruction = deserialize(
            InstructionSchema,
            CreateCompressedMemoLayout,
            Buffer.from(serializedData)
        );

        expect(deserializedInstruction.enumTag).to.equal(instructionData.enumTag);
        expect(deserializedInstruction.memo).to.equal(instructionData.memo);
        console.log("Memo instruction data matches original!");
    });
});
