use solana_program::pubkey::Pubkey;
use solana_sdk::pubkey;

// Program data
pub const SPL_ASSOCIATED_TOKEN: &[u8] =
    include_bytes!("programs/spl_associated-token-account-1.0.1.so");
pub const SPL_MEMO1: &[u8] = include_bytes!("programs/spl_memo-1.0.0.so");
pub const SPL_MEMO3: &[u8] = include_bytes!("programs/spl_memo-3.0.0.so");
pub const SPL_TOKEN: &[u8] = include_bytes!("programs/spl_token-3.1.0.so");

// Program IDs
pub const SPL_ASSOCIATED_TOKEN_PID: Pubkey =
    pubkey!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
pub const SPL_MEMO1_PID: Pubkey = pubkey!("Memo1UhkJRfHyvLMcVucJwxXeuD728EqVDDwQDxFMNo");
pub const SPL_MEMO3_PID: Pubkey = pubkey!("MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr");
pub const SPL_TOKEN_PID: Pubkey = pubkey!("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
pub const BPF_LOADER2_PID: Pubkey = pubkey!("BPFLoader2111111111111111111111111111111111");
pub const BPF_LOADER_UPGRADEABLE_PID: Pubkey =
    pubkey!("BPFLoaderUpgradeab1e11111111111111111111111");
pub const SYSVAR_PID: Pubkey = pubkey!("Sysvar1111111111111111111111111111111111111");
pub const SYSTEM_PID: Pubkey = pubkey!("11111111111111111111111111111111");

// Sysvar addresses
pub const SYSVAR_RENT_ADDRESS: Pubkey = pubkey!("SysvarRent111111111111111111111111111111111");
