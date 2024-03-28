mod memory_ex;

#[tokio::main]
async fn main() {
    memory_ex::ex_long_short_memory().await;
}
