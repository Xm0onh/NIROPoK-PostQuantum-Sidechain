use crate::transaction::Transaction;
use log::info;
use tokio::sync::mpsc::UnboundedSender;
use warp::Filter;

pub async fn start_rpc_server(rpc_sender: UnboundedSender<Transaction>) {
    // Define the RPC route on POST /rpc/transaction
    let rpc_route = warp::post()
        .and(warp::path("rpc"))
        .and(warp::path("transaction"))
        .and(warp::body::json())
        .and_then(move |txn: Transaction| {
            let rpc_sender = rpc_sender.clone();
            async move {
                rpc_sender
                    .send(txn)
                    .expect("Failed to send RPC transaction");
                Ok::<_, warp::Rejection>(warp::reply::json(&serde_json::json!({"status": "ok"})))
            }
        });

    // Bind to an ephemeral port
    let (addr, server) = warp::serve(rpc_route)
        .try_bind_ephemeral(([127, 0, 0, 1], 0))
        .expect("Failed to bind ephemeral RPC port");
    info!("RPC server running on {}", addr);
    server.await;
}
