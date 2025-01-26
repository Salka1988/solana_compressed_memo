/**
 * Run this script via:
 *    npx ts-node tests/test_compressed_memo.ts
 */
import {
    Connection,
    Keypair,
    PublicKey,
    Transaction,
    TransactionInstruction,
    SystemProgram,
    sendAndConfirmTransaction,
} from "@solana/web3.js";
import { serialize } from "borsh";
import * as fs from "node:fs";

// ------------------ Borsh Layout for the Instruction (Example) ------------------ //
/**
 * Suppose your Rust's ExtendedSPLMemoInstruction::CreateCompressedMemo
 * is Borsh-serialized as:
 *
 *   enumTag (u8)  -> for CreateCompressedMemo
 *   memoLen (u32) -> length of memo
 *   memoBytes (u8[]) -> the memo string
 *
 * Adjust this schema to match exactly how your Rust code serializes instructions!
 */
class CreateCompressedMemoLayout {
    enumTag: number;
    memo: string;

    constructor(fields: { enumTag: number; memo: string }) {
        this.enumTag = fields.enumTag;
        this.memo = fields.memo;
    }
}

// A Borsh schema for the instruction data.
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

(async () => {

    // 1) Read the Program ID from file
    const args = process.argv.slice(2);
    const programIdArg = args.find((arg) => arg.startsWith("--program-id="));
    if (!programIdArg) {
        throw new Error("Missing required --program-id argument.");
    }

    const programIdStr = programIdArg.split("=")[1];
    const programId = new PublicKey(programIdStr);
    console.log("Using Program ID:", programId.toBase58());

    // 1) Connect to local validator
    const connection = new Connection("http://127.0.0.1:8899", "confirmed");

    // 2) Create & fund payer
    const payer = Keypair.generate();
    const airdropSig = await connection.requestAirdrop(payer.publicKey, 1e9); // 1 SOL
    await connection.confirmTransaction(airdropSig);
    console.log("Payer created and funded:", payer.publicKey.toBase58());

    // 3) Prepare the Program and Accounts
    //    Replace this with your actual Program ID
    // const programId = new PublicKey("Fw6eh5oW7G8NdnkD4qDHWiQuGyZ1u48osomgJWBTfuTi");
    const newAccount = Keypair.generate();

    // 4) Build Instruction Data (Borsh)
    const memoStr = "Hello from TypeScript test!";
    const instructionData = new CreateCompressedMemoLayout({
        enumTag: 1, // e.g. if 0 => OriginalMemo, 1 => CreateCompressedMemo
        memo: memoStr,
    });

    const serializedData = serialize(InstructionSchema, instructionData);

    // 5) Construct the Transaction Instruction
    //    Typically your Rust expects:
    //      - payer as signer/writable
    //      - new account as signer/writable
    //      - system program as read-only
    //
    //    Because the Rust code calls system_instruction::create_account internally,
    //    we do *not* call SystemProgram.createAccount here to avoid "already in use."
    const ix = new TransactionInstruction({
        programId,
        keys: [
            { pubkey: payer.publicKey, isSigner: true, isWritable: true },
            { pubkey: newAccount.publicKey, isSigner: true, isWritable: true },
            { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        ],
        data: Buffer.from(serializedData),
    });

    // 6) No createAccountIx: the on-chain code does it.

    // 7) Send single Transaction with the one instruction
    const tx = new Transaction().add(ix);
    const txSig = await sendAndConfirmTransaction(connection, tx, [payer, newAccount]);
    console.log("Transaction Signature:", txSig);

    // 8) Verify the Data in newAccount
    const newAcctInfo = await connection.getAccountInfo(newAccount.publicKey);
    if (!newAcctInfo) {
        throw new Error("newAccount not found on chain.");
    }
    console.log("newAccount data length:", newAcctInfo.data.length);
    console.log("newAccount data bytes:", newAcctInfo.data);

    if (newAcctInfo.data.length === 32) {
        console.log("Success: The account data is 32 bytes as expected!");
    } else {
        console.warn("Unexpected account data length, got:", newAcctInfo.data.length);
    }

    console.log("TypeScript test completed successfully.");
})().catch((err) => {
    console.error("Test failed:", err);
    process.exit(1);
});
