mod memory_ex;
mod prompt_ex;
#[tokio::main]
async fn main() {
    // memory_ex::ex_long_short_memory().await;
    prompt_ex::ex_prompt_common().await;
}
