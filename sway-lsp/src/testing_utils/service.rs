use lspower::{ExitedError, LspService, MessageStream};
use serde_json::Value;
use tower_test::mock::Spawn;

pub async fn send(service: &mut Spawn<LspService>, request: &Value) -> Result<Option<Value>, ExitedError> {
    let request = serde_json::from_value(request.clone()).unwrap();
    let response = service.call(request).await?;
    let response = response.and_then(|x| serde_json::to_value(x).ok());
    Ok(response)
}

pub fn spawn() -> anyhow::Result<(Spawn<LspService>, MessageStream)> {
    let (service, messages) = LspService::new(|client| {
        crate::server::Backend::new(client)
    });
    Ok((Spawn::new(service), messages))
}
