mod grpc;
// #[cfg(feature = "grpc")]
mod proto;
mod py_runtime;

fn main() {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(8)
        .enable_all()
        .build()
        .expect("tokio runtime build failed");

    rt.block_on(async {
        grpc::server::Server::default()
            .run("0.0.0.0:50001")
            .await
            .unwrap();
    });
}