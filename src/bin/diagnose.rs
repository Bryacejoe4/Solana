use solana_client::rpc_client::RpcClient;
use solana_sdk::pubkey::Pubkey;
use solana_sdk::signature::{read_keypair_file, Signer};
use std::str::FromStr;

fn main() {
    let rpc = RpcClient::new("https://api.mainnet-beta.solana.com".to_string());

    let program_id = Pubkey::from_str("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P").unwrap();
    let mint = Pubkey::from_str("2TBYaBwCAL1NLhExcLecmAcSc4CAmw59cMSSfsqeC1yJ").unwrap();
    let global = Pubkey::from_str("4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf").unwrap();
    let fee_recipient = Pubkey::from_str("CebN5WGQ4jvEPvsVU4EoHEpgzq1VV7AbicfhtW4xC9iM").unwrap();
    let event_authority = Pubkey::from_str("Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1").unwrap();
    let token_program = Pubkey::from_str("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA").unwrap();
    let ata_program = Pubkey::from_str("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL").unwrap();

    // Load signer
    let kp_path = shellexpand::tilde("~/.config/solana/id.json").to_string();
    let signer_pubkey = match read_keypair_file(&kp_path) {
        Ok(kp) => kp.pubkey(),
        Err(e) => { println!("ERROR: Cannot load keypair at {}: {}", kp_path, e); return; }
    };

    // Derive PDAs
    let (bonding_curve, _) = Pubkey::find_program_address(&[b"bonding-curve", mint.as_ref()], &program_id);
    let (associated_bonding_curve, _) = Pubkey::find_program_address(
        &[bonding_curve.as_ref(), token_program.as_ref(), mint.as_ref()],
        &ata_program,
    );
    let (associated_user, _) = Pubkey::find_program_address(
        &[signer_pubkey.as_ref(), token_program.as_ref(), mint.as_ref()],
        &ata_program,
    );

    println!("=== PUMP.FUN ACCOUNT DIAGNOSTIC ===\n");
    println!("Signer:    {}", signer_pubkey);
    println!("Mint:      {}", mint);
    println!("Bonding:   {}", bonding_curve);
    println!("ATA Bond:  {}", associated_bonding_curve);
    println!("ATA User:  {}", associated_user);
    println!("");

    let accounts_to_check: Vec<(&str, Pubkey)> = vec![
        ("GLOBAL", global),
        ("FEE_RECIPIENT", fee_recipient),
        ("MINT", mint),
        ("BONDING_CURVE", bonding_curve),
        ("ASSOC_BONDING_CURVE", associated_bonding_curve),
        ("ASSOC_USER", associated_user),
        ("SIGNER", signer_pubkey),
        ("EVENT_AUTHORITY", event_authority),
    ];

    for (name, pubkey) in accounts_to_check {
        match rpc.get_account(&pubkey) {
            Ok(account) => {
                println!("✅ {} ({}) - EXISTS, owner: {}, lamports: {}", name, pubkey, account.owner, account.lamports);
            }
            Err(e) => {
                println!("❌ {} ({}) - NOT FOUND: {}", name, pubkey, e);
            }
        }
    }
    println!("\n=== DONE ===");
}
