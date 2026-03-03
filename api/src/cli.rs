use common::types::{BuyRequest, SellRequest};
use dialoguer::{theme::ColorfulTheme, Input, Select};
use reqwest::Client;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = Client::new();
    let modes = &["Buy", "Sell", "Cancel", "Quit"];
    
    loop {
        let selection = Select::with_theme(&ColorfulTheme::default())
            .with_prompt("Select action")
            .default(0)
            .items(&modes[..])
            .interact()?;

        match selection {
            0 => {
                let token_mint: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Token Mint")
                    .interact_text()?;
                
                let amount_sol: f64 = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Amount (SOL)")
                    .default(0.0001)
                    .interact_text()?;
                    
                let req = BuyRequest {
                    launchpad: "pumpfun".to_string(),
                    token_mint,
                    amount_sol,
                    max_slippage_bps: Some(1000),
                    token_program: None,
                    creator: None,
                };
                
                println!("Submitting Buy...");
                let res = client.post("http://localhost:8080/v1/trade/buy")
                    .json(&req)
                    .send()
                    .await?;
                println!("Response: {}", res.text().await?);
            },
            1 => {
                let token_mint: String = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Token Mint")
                    .interact_text()?;
                
                let amount_tokens: u64 = Input::with_theme(&ColorfulTheme::default())
                    .with_prompt("Amount (Tokens)")
                    .default(1000000)
                    .interact_text()?;
                    
                let req = SellRequest {
                    launchpad: "pumpfun".to_string(),
                    token_mint,
                    amount_tokens,
                    max_slippage_bps: Some(1000),
                    token_program: None,
                    creator: None,
                };
                
                println!("Submitting Sell...");
                let res = client.post("http://localhost:8080/v1/trade/sell")
                    .json(&req)
                    .send()
                    .await?;
                println!("Response: {}", res.text().await?);
            },
            _ => break,
        }
    }
    Ok(())
}
