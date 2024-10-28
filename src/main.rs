use tokio::{
    io::{stdin, AsyncBufReadExt, BufReader},
    select, spawn,
    sync::mpsc,
    time::sleep,
};
use libp2p::{
    swarm::{SwarmBuilder, SwarmEvent},
    Multiaddr,
};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};
use futures::stream::StreamExt;

mod wallet;
mod transaction;
mod p2p;
mod block;
mod blockchain;
mod mempool;
mod account;
mod stake;
mod validator;

use log::info;
#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    info!("Starting the new Peer, {}", p2p::PEER_ID.clone());
    let (init_sender, mut init_rcv) = mpsc::unbounded_channel::<bool>();

    let wallet = wallet::Wallet::new().unwrap();

    let behavior = p2p::AppBehaviour::new().await;
    let transport = libp2p::tokio_development_transport(p2p::KEYS.clone()).expect("Failed to create transport");

    let mut swarm = SwarmBuilder::with_tokio_executor(transport, behavior, p2p::PEER_ID.clone()).build();
    
    let listen_addr: Multiaddr = "/ip4/0.0.0.0/tcp/0"
    .parse()
    .expect("Failed to parse listen address");

    swarm.listen_on(listen_addr).expect("Failed to listen on address");
    
     // Send an init event after 1 second
     let init_sender_clone = init_sender.clone();
     spawn(async move {
         sleep(Duration::from_secs(1)).await;
         info!("sending init event");
         init_sender_clone.send(true).expect("can send init event");
     });

     let mut planner = periodic::Planner::new();
     planner.start();
      //TODO  Run mining every second
     /*
     
     */
     loop {
        // start listening for events
       select! {
        event = swarm.select_next_some() => {
            match event {
                SwarmEvent::NewListenAddr { address, .. } => {
                    info!("Listening on {:?}", address);
                    
                }
                _ => {}
            }
        },
       }


     }

}
