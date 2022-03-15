use lspower::{LspService, Server};

mod capabilities;
mod core;
mod server;
mod sway_config;
mod utils;
mod testing_utils;
use server::Backend;

mod tests;

pub async fn start() {
    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, messages) = LspService::new(Backend::new);
    Server::new(stdin, stdout)
        .interleave(messages)
        .serve(service)
        .await;
}
