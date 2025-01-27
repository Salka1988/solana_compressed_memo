use borsh::{BorshDeserialize, BorshSerialize};

#[derive(BorshSerialize, BorshDeserialize, Debug)]
pub enum ExtendedSPLMemoInstruction {
    /// Original SPL Memo functionality (for reference).
    OriginalMemo { memo: String },

    /// Create a compressed account and store the memo in it.
    CreateCompressedMemo { memo: String },
}
