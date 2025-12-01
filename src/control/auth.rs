use super::types::*;
use futures::future::BoxFuture;
use std::collections::HashMap;
use std::sync::Arc;
use tower::Service;

/// Pluggable authentication/authorization provider.
pub trait AuthProvider: Send + Sync {
    /// Unique name of this provider.
    fn name(&self) -> &'static str;

    /// Verify credentials; returns context on success.
    fn authenticate(
        &self,
        meta: &CommandMeta,
        auth: Option<&AuthPayload>,
    ) -> Result<AuthContext, AuthError>;

    /// Optional authorization using the command label.
    ///
    /// # Default Implementation
    ///
    /// Returns `Err(AuthError::Unauthorized)` (fail-closed). Implementors MUST override this
    /// to explicitly grant access.
    fn authorize(
        &self,
        _ctx: &AuthContext,
        _label: &str,
        _meta: &CommandMeta,
    ) -> Result<(), AuthError> {
        Err(AuthError::Unauthorized("default authorize denies all".into()))
    }
}

/// Registry that tries providers in order.
#[derive(Clone)]
pub struct AuthRegistry {
    providers: Vec<Arc<dyn AuthProvider>>,
    mode: AuthMode,
}

/// Strategy for combining multiple auth providers.
#[derive(Clone, Copy, Debug)]
pub enum AuthMode {
    /// First provider that authenticates wins.
    First,
    /// All providers must succeed; principal is taken from the first successful provider and
    /// attributes from subsequent providers are merged (later attributes overwrite earlier keys).
    All,
}

impl AuthRegistry {
    /// Create a new registry with the given mode.
    pub fn new(mode: AuthMode) -> Self {
        Self { providers: Vec::new(), mode }
    }

    /// Register an auth provider.
    pub fn register(&mut self, provider: Arc<dyn AuthProvider>) {
        self.providers.push(provider);
    }

    /// Authenticate a command envelope using registered providers.
    pub fn authenticate<C>(&self, env: &CommandEnvelope<C>) -> Result<AuthContext, AuthError>
    where
        C: CommandLabel + Clone,
    {
        match self.mode {
            AuthMode::First => {
                let mut last_err = None;
                for p in &self.providers {
                    match p.authenticate(&env.meta, env.auth.as_ref()) {
                        Ok(ctx) => {
                            // If a provider authenticates but denies authorization, stop immediately
                            // to prevent later providers from overriding an explicit deny.
                            match p.authorize(&ctx, env.cmd.label(), &env.meta) {
                                Ok(()) => return Ok(ctx),
                                Err(e) => return Err(e),
                            }
                        }
                        Err(e) => last_err = Some(e),
                    }
                }
                Err(last_err.unwrap_or(AuthError::Unauthenticated("no providers".into())))
            }
            AuthMode::All => {
                let mut combined: Option<AuthContext> = None;
                for p in &self.providers {
                    let ctx = p.authenticate(&env.meta, env.auth.as_ref())?;
                    p.authorize(&ctx, env.cmd.label(), &env.meta)?;
                    combined = Some(match combined {
                        None => ctx,
                        Some(mut agg) => {
                            // Prefer principal from the first successful provider; merge attributes.
                            agg.attributes.extend(ctx.attributes.into_iter());
                            agg
                        }
                    });
                }
                combined.ok_or(AuthError::Unauthenticated("no providers".into()))
            }
        }
    }
}

/// Authorization layer wraps an inner service and preserves auth registry for later use.
#[derive(Clone)]
pub struct AuthorizationLayer {
    registry: Arc<AuthRegistry>,
}

impl AuthorizationLayer {
    /// Create a new authorization layer with the given registry.
    pub fn new(registry: AuthRegistry) -> Self {
        Self { registry: Arc::new(registry) }
    }
}

/// Service that applies authorization checks before forwarding commands.
#[derive(Clone)]
pub struct AuthorizationService<S> {
    inner: S,
    registry: Arc<AuthRegistry>,
}

impl<S> tower_layer::Layer<S> for AuthorizationLayer {
    type Service = AuthorizationService<S>;

    fn layer(&self, inner: S) -> Self::Service {
        AuthorizationService { inner, registry: self.registry.clone() }
    }
}

impl<S, C> Service<CommandEnvelope<C>> for AuthorizationService<S>
where
    C: CommandLabel + Clone + Send + Sync + 'static,
    S: Service<CommandEnvelope<C>, Response = CommandResult, Error = CommandError>
        + Clone
        + Send
        + 'static,
    S::Future: Send + 'static,
{
    type Response = CommandResult;
    type Error = CommandError;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(
        &mut self,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Result<(), CommandError>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: CommandEnvelope<C>) -> Self::Future {
        let registry = self.registry.clone();
        let inner = self.inner.clone();
        Box::pin(async move {
            registry.authenticate(&req).map_err(CommandError::Auth)?;
            // inner is moved into the future; Service::call requires mutable access,
            // but the binding itself doesn't need to be mut for the move.
            // However, we must ensure we can call .call(&mut inner).
            // Since inner is owned by the future, we need to make it mutable inside?
            // Let's trust the compiler and remove `mut`.
            // If it fails to compile because `inner` is immutable, we'll need to shadow it:
            // let mut inner = inner;
            let mut inner = inner;
            inner.call(req).await
        })
    }
}

/// Passthrough provider (dev/testing).
///
/// # ⚠️ SECURITY WARNING
///
/// This provider grants access to **everyone** as "anonymous".
/// **DO NOT USE IN PRODUCTION**. It effectively disables authentication.
pub struct PassthroughAuth;
impl AuthProvider for PassthroughAuth {
    fn name(&self) -> &'static str {
        "passthrough"
    }
    fn authenticate(
        &self,
        _meta: &CommandMeta,
        _auth: Option<&AuthPayload>,
    ) -> Result<AuthContext, AuthError> {
        Ok(AuthContext {
            principal: "anonymous".into(),
            provider: self.name(),
            attributes: HashMap::new(),
        })
    }

    fn authorize(
        &self,
        _ctx: &AuthContext,
        _label: &str,
        _meta: &CommandMeta,
    ) -> Result<(), AuthError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use tower_layer::Layer;
    use tower_service::Service;

    #[derive(Clone)]
    struct AllowAuth {
        name: &'static str,
        principal: &'static str,
        attrs: HashMap<String, String>,
    }

    impl AuthProvider for AllowAuth {
        fn name(&self) -> &'static str {
            self.name
        }
        fn authenticate(
            &self,
            _meta: &CommandMeta,
            _auth: Option<&AuthPayload>,
        ) -> Result<AuthContext, AuthError> {
            Ok(AuthContext {
                principal: self.principal.to_string(),
                provider: self.name,
                attributes: self.attrs.clone(),
            })
        }
        fn authorize(
            &self,
            _ctx: &AuthContext,
            _label: &str,
            _meta: &CommandMeta,
        ) -> Result<(), AuthError> {
            Ok(())
        }
    }

    #[derive(Clone)]
    struct DummyCmd;
    impl CommandLabel for DummyCmd {
        fn label(&self) -> &str {
            "dummy"
        }
    }

    #[test]
    fn auth_mode_all_merges_attributes_and_keeps_first_principal() {
        let mut reg = AuthRegistry::new(AuthMode::All);
        let mut a1_attrs = HashMap::new();
        a1_attrs.insert("role".into(), "admin".into());
        let mut a2_attrs = HashMap::new();
        a2_attrs.insert("scope".into(), "write".into());
        a2_attrs.insert("role".into(), "user".into()); // should overwrite same key

        reg.register(Arc::new(AllowAuth { name: "p1", principal: "alice", attrs: a1_attrs }));
        reg.register(Arc::new(AllowAuth { name: "p2", principal: "bob", attrs: a2_attrs }));

        let env = CommandEnvelope {
            cmd: DummyCmd,
            auth: None,
            meta: CommandMeta { id: "1".into(), correlation_id: None, timestamp_millis: None },
        };

        let ctx = reg.authenticate(&env).expect("auth ok");
        assert_eq!(ctx.principal, "alice", "first principal should win");
        assert_eq!(ctx.provider, "p1");
        assert_eq!(ctx.attributes.get("role").unwrap(), "user"); // overwritten by later provider
        assert_eq!(ctx.attributes.get("scope").unwrap(), "write");
    }

    #[test]
    fn authorization_service_forwards_poll_ready_errors() {
        #[derive(Clone)]
        struct FailReady;
        impl Service<CommandEnvelope<DummyCmd>> for FailReady {
            type Response = CommandResult;
            type Error = CommandError;
            type Future = futures::future::Ready<Result<Self::Response, Self::Error>>;
            fn poll_ready(
                &mut self,
                _cx: &mut std::task::Context<'_>,
            ) -> std::task::Poll<Result<(), Self::Error>> {
                std::task::Poll::Ready(Err(CommandError::Handler("not ready".into())))
            }
            fn call(&mut self, _req: CommandEnvelope<DummyCmd>) -> Self::Future {
                futures::future::ready(Ok(CommandResult::Ack))
            }
        }

        let layer = AuthorizationLayer::new(AuthRegistry::new(AuthMode::First));
        let mut svc = layer.layer(FailReady);
        let mut cx = std::task::Context::from_waker(futures::task::noop_waker_ref());
        let res = svc.poll_ready(&mut cx);
        match res {
            std::task::Poll::Ready(Err(CommandError::Handler(msg))) => {
                assert_eq!(msg, "not ready");
            }
            other => panic!("expected handler error, got {:?}", other),
        }
    }
}
