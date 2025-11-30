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
    fn authorize(
        &self,
        _ctx: &AuthContext,
        _label: &str,
        _meta: &CommandMeta,
    ) -> Result<(), AuthError> {
        Ok(())
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
    /// All providers must succeed.
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
                let mut last_ctx = None;
                for p in &self.providers {
                    let ctx = p.authenticate(&env.meta, env.auth.as_ref())?;
                    p.authorize(&ctx, env.cmd.label(), &env.meta)?;
                    last_ctx = Some(ctx);
                }
                last_ctx.ok_or(AuthError::Unauthenticated("no providers".into()))
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
        // This service clones the inner per-call, so it is always ready.
        let _ = cx;
        std::task::Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: CommandEnvelope<C>) -> Self::Future {
        let registry = self.registry.clone();
        let mut inner = self.inner.clone();
        Box::pin(async move {
            registry.authenticate(&req).map_err(CommandError::Auth)?;
            use tower::ServiceExt;
            inner.ready_oneshot().await?.call(req).await
        })
    }
}

/// Passthrough provider (dev/testing).
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
}
