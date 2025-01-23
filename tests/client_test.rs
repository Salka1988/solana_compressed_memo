// import {
// Connection,
// Keypair,
// PublicKey,
// sendAndConfirmTransaction,
// SystemProgram,
// Transaction,
// TransactionInstruction,
// } from "@solana/web3.js";
// import { ExtendedSPLMemoInstruction } from "../target/types/extanded_spl_idl";
// // ^ If you generate an IDL or define some TS type for your instructions
// //   Alternatively, just build data buffers manually.
//
// import { serialize } from "borsh";
//
// (async () => {
// // 1) Setup
// const connection = new Connection("http://127.0.0.1:8899", "confirmed");
// const payer = Keypair.generate();
// await connection.requestAirdrop(payer.publicKey, 1_000_000_000); // 1 SOL
//
// // 2) Prepare new account and instruction data
// const newAccount = Keypair.generate();
// const memoStr = "Hello from TypeScript test!";
// // Build instruction data:
// // This must match the Rust Borsh format of ExtendedSPLMemoInstruction::CreateCompressedMemo
// const instructionData = serialize(
// // Borsh schema for ExtendedSPLMemoInstruction if you define one in TS
// // or manually encode the enum tag + memo string:
// {
// ExtendedSPLMemoInstruction: {
// kind: "struct",
// fields: [
// ["enumTag", "u8"], // 1 for CreateCompressedMemo if that's how you define the enum
// ["memo", "string"],
// ],
// },
// },
// {
// enumTag: 1, // 0 => OriginalMemo, 1 => CreateCompressedMemo
// memo: memoStr,
// }
// );
//
// // 3) Create TransactionInstruction
// const programId = new PublicKey("<Your_Program_ID>");
// const createCompressedMemoIx = new TransactionInstruction({
// programId,
// keys: [
// { pubkey: payer.publicKey, isSigner: true, isWritable: false },
// { pubkey: newAccount.publicKey, isSigner: false, isWritable: true },
// ],
// data: Buffer.from(instructionData),
// });
//
// // 4) Send transaction
// let tx = new Transaction().add(
// SystemProgram.createAccount({
// fromPubkey: payer.publicKey,
// newAccountPubkey: newAccount.publicKey,
// lamports: 0, // the program will handle exact rent-exempt lamports
// space: 0,    // the program will handle the exact space requirement
// programId,
// }),
// createCompressedMemoIx
// );
//
// tx.sign(payer, newAccount);
//
// const txSig = await sendAndConfirmTransaction(connection, tx, [payer, newAccount]);
// console.log("Transaction Signature:", txSig);
//
// // 5) Verify the new account data
// const newAcctInfo = await connection.getAccountInfo(newAccount.publicKey);
// console.log("Compressed Account Data:", newAcctInfo?.data);
// })();
