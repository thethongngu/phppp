use phppp::server::run_server;

#[tokio::main]
async fn main() {
    run_server().await;
}
