use borsh_derive::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum ExtendedSPLMemoInstruction {
    /// Original SPL Memo functionality (for reference).
    /// Stores a string in transaction logs, no account data changes.
    ///   0. `[signer]` The payer (or any signer)
    ///   1. `[]` (optional) Additional accounts read
    ///   data: the memo string
    OriginalMemo {
        memo: String,
    },

    /// Create a compressed account and store the memo in it.
    ///   0. `[signer]` Payer for account creation
    ///   1. `[writable]` The newly created account (PDA or fresh key)
    ///   data: the memo string
    CreateCompressedMemo {
        memo: String,
    },
}
