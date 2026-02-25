use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

fn main() {
    let program = Pubkey::from_str("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P").unwrap();
    let acc13 = Pubkey::from_str("FhZTNKjqYjsjH22U99W2zXtcfwA13fFcHi2wF58mWXTN").unwrap();
    let user = Pubkey::from_str("qRjNSMajxpKzdHV6Ya7fpUAW5sqMTfHwKvpxVtXM1se").unwrap();
    let mint = Pubkey::from_str("7QXDAUDwHiBTpjXaDKieotMWGF59wDddQyA8AF2Qpump").unwrap();

    let mut found = false;
    let prefixes = [
        "user_volume_accumulator", "global_volume_accumulator", "volume_accumulator", 
        "user_acc", "user", "accumulator"
    ];

    for pf in prefixes.iter() {
        let (pda, _) = Pubkey::find_program_address(&[pf.as_bytes(), user.as_ref()], &program);
        if pda == acc13 { println!("MATCH! Target depends on USER and prefix '{}'", pf); found = true;}
        let (pda, _) = Pubkey::find_program_address(&[pf.as_bytes(), mint.as_ref()], &program);
        if pda == acc13 { println!("MATCH! Target depends on MINT and prefix '{}'", pf); found = true;}
    }

    if !found {
        println!("No naive match. Trying token program dependencies or empty seeds...");
        let (pda, _) = Pubkey::find_program_address(&[user.as_ref()], &program);
        if pda == acc13 { println!("MATCH! Target depends ONLY on USER."); }
        let (pda, _) = Pubkey::find_program_address(&[mint.as_ref()], &program);
        if pda == acc13 { println!("MATCH! Target depends ONLY on MINT."); }
    }
}
