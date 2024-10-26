
mod wallet;
mod transaction;
mod p2p;
mod block;


fn main() {
    let wallet = wallet::Wallet::new().unwrap();
    println!("{:?}", hex::encode(wallet.keypair.public.to_bytes()));
    println!("{:?}", hex::encode(wallet.keypair.secret.to_bytes()));
    println!("Hello, world!");
}
