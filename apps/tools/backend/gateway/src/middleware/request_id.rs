use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicU64, Ordering};
use std::task::{Context, Poll};

use axum::{extract::Request, response::Response};
use tower::{Layer, Service};
use uuid::Uuid;

/// Middleware that adds a unique X-Request-Id header to every request.
#[derive(Clone, Default)]
pub struct RequestIdLayer;

impl<S> Layer<S> for RequestIdLayer {
    type Service = RequestIdMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        RequestIdMiddleware {
            inner,
            counter: AtomicU64::new(0),
        }
    }
}

pub struct RequestIdMiddleware<S> {
    inner: S,
    counter: AtomicU64,
}

impl<S: Clone> Clone for RequestIdMiddleware<S> {
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            counter: AtomicU64::new(self.counter.load(Ordering::Relaxed)),
        }
    }
}

impl<S, ReqBody, ResBody> Service<Request<ReqBody>> for RequestIdMiddleware<S>
where
    S: Service<Request<ReqBody>, Response = Response<ResBody>>,
    S::Future: Send + 'static,
    S::Error: 'static,
    ReqBody: Send + 'static,
    ResBody: Default + Send + 'static,
{
    type Response = Response<ResBody>;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, req: Request<ReqBody>) -> Self::Future {
        let request_id = Uuid::new_v4().to_string();
        let (mut parts, body) = req.into_parts();
        parts
            .headers
            .insert("x-request-id", request_id.parse().unwrap());

        let req = Request::from_parts(parts, body);
        let fut = self.inner.call(req);

        Box::pin(async move {
            let mut response: Response<ResBody> = fut.await?;
            response
                .headers_mut()
                .insert("x-request-id", request_id.parse().unwrap());
            Ok(response)
        })
    }
}