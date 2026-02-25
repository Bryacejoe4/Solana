use solana_sdk::pubkey::Pubkey;
use std::str::FromStr;

fn main() {
    let program_id = Pubkey::from_str("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P").unwrap();
    let signer = Pubkey::from_str("GTAbrrpHy63U1ipynjb4jGppyo2Te6LLKBzH3jsN88Nk").unwrap();
    let target = "Hj5SbnkSKHX1ifC7GVrJFqK1PVPWnX5HqEia4Sa59wfp";

    let words: Vec<&[u8]> = vec![
        b"user",
        b"user_volume_accumulator",
        b"user-volume-accumulator",
        b"user_volume",
        b"user-volume",
        b"volume",
        b"accumulator",
        b"pump",
        b"pumpfun",
    ];

    for w in words.iter() {
        let (pda, _) = Pubkey::find_program_address(&[w, signer.as_ref()], &program_id);
        if pda.to_string() == target {
            println!("FOUND SEED: {}", std::str::from_utf8(w).unwrap());
            return;
        }
    }
    
    let (pda, _) = Pubkey::find_program_address(&[signer.as_ref()], &program_id);
    if pda.to_string() == target {
        println!("FOUND SEED: just signer");
        return;
    }

    println!("Not found.");
}
