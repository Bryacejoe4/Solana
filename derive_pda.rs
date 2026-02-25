use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

fn main() {
    let program_id = Pubkey::from_str("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P").unwrap();
    let mint = Pubkey::from_str("2TBYaBwCAL1NLhExcLecmAcSc4CAmw59cMSSfsqeC1yJ").unwrap();
    let token_program = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
    let ata_program = Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL").unwrap();

    let (bonding_curve, _) = Pubkey::find_program_address(&[b"bonding-curve", mint.as_ref()], &program_id);
    let (assoc_bonding, _) = Pubkey::find_program_address(
        &[bonding_curve.as_ref(), token_program.as_ref(), mint.as_ref()],
        &ata_program,
    );

    println!("BONDING_CURVE={}", bonding_curve);
    println!("ASSOC_BONDING={}", assoc_bonding);
}
