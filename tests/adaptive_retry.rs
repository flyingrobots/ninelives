use ninelives::adaptive::Adaptive;
use ninelives::{Backoff, Jitter, RetryPolicy};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;
use tower::{Service, ServiceBuilder, ServiceExt};

#[tokio::test]
async fn retry_respects_live_max_attempts() {
    // Build policy with initial max_attempts = 1
    let policy = RetryPolicy::<TestError>::builder()
        .max_attempts(1)
        .backoff(Backoff::constant(Duration::from_millis(1)))
        .with_jitter(Jitter::full())
        .build()
        .unwrap();

    // Extract adaptive handle (future API to be provided by policy/layer)
    let max_handle: Adaptive<usize> = policy.adaptive_max_attempts();

    let layer = policy.clone().into_layer();
    let svc = TestService::new(3);
    let mut wrapped = ServiceBuilder::new().layer(layer).service(svc.clone());

    // With max_attempts=1 should fail (needs 3 attempts)
    let res = wrapped.ready().await.unwrap().call(()).await;
    assert!(res.is_err());

    // Update max attempts live and try again
    max_handle.set(3);
    let mut wrapped = ServiceBuilder::new().layer(policy.into_layer()).service(svc);
    let res = wrapped.ready().await.unwrap().call(()).await;
    assert!(res.is_ok());
}

#[derive(Clone)]
struct TestService {
    target: usize,
    counter: Arc<AtomicUsize>,
}

impl TestService {
    fn new(target: usize) -> Self {
        Self { target, counter: Arc::new(AtomicUsize::new(0)) }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("test error")]
struct TestError;

impl Service<()> for TestService {
    type Response = ();
    type Error = TestError;
    type Future = futures::future::Ready<Result<(), TestError>>;

    fn poll_ready(
        &mut self,
        _cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), Self::Error>> {
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, _req: ()) -> Self::Future {
        let n = self.counter.fetch_add(1, Ordering::SeqCst) + 1;
        if n >= self.target {
            futures::future::ready(Ok(()))
        } else {
            futures::future::ready(Err(TestError))
        }
    }
}
