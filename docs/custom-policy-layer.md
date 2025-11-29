# Custom Policy Layer (Advanced)

This guide shows how to build your own Tower service layer, compose it with the Nine Lives algebra (`+`, `|`, `&`), and surface it through the control plane.

## 1) Implement a Tower `Service`

```rust
use tower_service::Service;

#[derive(Clone)]
pub struct HeaderTagger;

impl<ServiceRequest> Service<ServiceRequest> for HeaderTagger
where
    ServiceRequest: Clone + Send + 'static,
{
    type Response = ServiceRequest;
    type Error = std::io::Error;
    type Future = futures::future::Ready<Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut std::task::Context<'_>) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: ServiceRequest) -> Self::Future {
        // customize the request here (e.g., tag headers, inject tracing ids)
        futures::future::ready(Ok(req))
    }
}
```

## 2) Wrap it in the algebra

```rust
use ninelives::Policy;
use tower::{ServiceBuilder, ServiceExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let layer = Policy(HeaderTagger)
        + Policy(ninelives::TimeoutLayer::new(std::time::Duration::from_secs(1))?);

    let mut svc = ServiceBuilder::new()
        .layer(layer)
        .service_fn(|req: String| async move { Ok::<_, std::io::Error>(req) });

    svc.ready().await?.call("hello".to_string()).await?;
    Ok(())
}
```

## 3) Expose via the control plane (optional)

1. Register any dynamic knobs in the config registry (e.g., tagging behavior flags) via `DefaultConfigRegistry`.
2. Inject the registry and your custom layer into the control-plane handler when constructing the service stack.

## 4) Testing

- Unit-test the layer in isolation (poll_ready, call).
- Integration-test it composed in the algebra: verify ordering with `+`, fallback with `|`, and race behavior with `&`.

## 5) Plug-in surface

- If you need runtime mutability, register your knobs with `DefaultConfigRegistry` so they can be adjusted via `WriteConfig` commands.
- For external observability, emit tracing spans/events inside your layer; Nine Lives telemetry will propagate spans through composed layers.

This pattern keeps custom behavior modular while staying compatible with the existing Nine Lives DSL and control-plane tooling.
