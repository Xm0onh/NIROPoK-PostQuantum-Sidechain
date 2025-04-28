use niropok_pq_sidechain::{
    accounts::Account,
    transaction::{Transaction, TransactionType},
    wallet::Wallet,
};
use reqwest;
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create a new wallet (in practice, you'd load an existing one)
    let mut sender_wallet = Wallet::new()?;
    let sender_address = sender_wallet.get_public_key().to_string();
    println!("Sender address: {}", sender_address);

    // Create sender account
    let sender = Account {
        address: sender_address,
    };

    let recipient = Account {
        address: "REPLACE_WITH_RECIPIENT_ADDRESS".to_string(),
    };

    // Create a new transaction
    let transaction = Transaction::new(
        &mut sender_wallet,
        sender,
        recipient,
        100.0, // amount to send
        0,     // fee
        TransactionType::TRANSACTION,
    )?;

    // Serialize the transaction to JSON
    let json = serde_json::to_string(&transaction)?;
    println!("Transaction JSON: {}", json);

    // Send the transaction to the RPC endpoint
    // Note: The port number will be shown in the node's logs when it starts
    let rpc_url = "http://127.0.0.1:55544/rpc/transaction";

    let client = reqwest::Client::new();
    let response = client
        .post(rpc_url)
        .header("Content-Type", "application/json")
        .body(json)
        .send()
        .await?;

    println!("Response status: {}", response.status());
    println!("Response body: {}", response.text().await?);

    Ok(())
}
