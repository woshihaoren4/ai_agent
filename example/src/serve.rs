mod agent_serve;
mod proto;
mod tools;

#[tokio::main]
async fn main() {
    agent_serve::start("0.0.0.0:50002").await;
}
