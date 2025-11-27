#![allow(missing_docs, unused_mut)]

use ninelives::control::{AuthMode, AuthRegistry, PassthroughAuth};
use ninelives::AuthorizationLayer;
use std::future::Ready;
use std::task::{Context, Poll};
use tower::{Layer, Service, ServiceExt};

#[derive(Clone)]
struct Echo;

impl Service<u32> for Echo {
    type Response = u32;
    type Error = std::convert::Infallible;
    type Future = Ready<Result<u32, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: u32) -> Self::Future {
        std::future::ready(Ok(req))
    }
}

#[tokio::test]
async fn authorization_layer_compiles_and_passes_through() {
    let mut registry = AuthRegistry::new(AuthMode::First);
    registry.register(std::sync::Arc::new(PassthroughAuth));

    let layer = AuthorizationLayer::new(registry);
    let mut svc = layer.layer(Echo);

    let mut ready = svc.ready().await.unwrap();
    let out = ready.call(7).await.unwrap();
    assert_eq!(out, 7);
}
