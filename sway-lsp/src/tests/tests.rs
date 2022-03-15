use serde_json::Value;
use crate::testing_utils as testing;
use crate::{assert_exchange, assert_status};

#[tokio::test]
async fn exit() -> anyhow::Result<()> {
    let service = &mut testing::service::spawn()?.0;

    // send "initialize" request
    assert_status!(service, Ok(()));
    let request = &testing::lsp::initialize::request();
    let response = Some(testing::lsp::initialize::response());
    assert_exchange!(service, request, Ok(response));

    // send "initialized" notification
    assert_status!(service, Ok(()));
    let notification = &testing::lsp::initialized::notification();
    let status = None::<Value>;
    assert_exchange!(service, notification, Ok(status));

    // send "exit" notification
    assert_status!(service, Ok(()));
    let notification = &testing::lsp::exit::notification();
    let status = None::<Value>;
    assert_exchange!(service, notification, Ok(status));

    // send "textDocument/didOpen" notification; should error
    assert_status!(service, Err(lspower::ExitedError));
    let notification = &{
        let uri = url::Url::parse("inmemory::///test")?;
        let language_id = "wasm.wat";
        let text = String::from("");
        testing::lsp::text_document::did_open::notification(&uri, language_id, 1, text)
    };
    let status = lspower::ExitedError;
    assert_exchange!(service, notification, Err(status));

    Ok(())
}

#[tokio::test]
async fn initialize() -> anyhow::Result<()> {
    let service = &mut testing::service::spawn()?.0;
    // send "initialize" request
    assert_status!(service, Ok(()));
    let request = &testing::lsp::initialize::request();
    let response = Some(testing::lsp::initialize::response());
    assert_exchange!(service, request, Ok(response));

    Ok(())
}


#[tokio::test]
async fn initialized() -> anyhow::Result<()> {
    let service = &mut testing::service::spawn()?.0;

    // send "initialize" request
    assert_status!(service, Ok(()));
    let request = &testing::lsp::initialize::request();
    let response = Some(testing::lsp::initialize::response());
    assert_exchange!(service, request, Ok(response));
    
    // send "initialized" notification
    assert_status!(service, Ok(()));
    let notification = &testing::lsp::initialized::notification();
    let status = None;
    assert_exchange!(service, notification, Ok(status));
    

    Ok(())
}


#[macro_export]
macro_rules! assert_status {
    ($service:expr, $status:expr) => {
        assert_eq!($service.poll_ready(), std::task::Poll::Ready($status));
    };
}

#[macro_export]
macro_rules! assert_exchange {
    ($service:expr, $request:expr, $response:expr) => {
        assert_eq!(testing::service::send($service, $request).await, $response);
    };
}
