use std::{future::Future, pin::Pin, sync::Mutex};


pub struct SendFuture<F: Future> {
    inner: Mutex<Pin<Box<F>>>,
}

impl<F: Future> From<F> for SendFuture<F> {
    fn from(value: F) -> Self {
        Self {
            inner: Mutex::new(Box::pin(value))
        }
    }
}

unsafe impl<F: Future> Send for SendFuture<F> {}

impl<F: Future> Future for SendFuture<F> {
    type Output = F::Output;

    fn poll(self: std::pin::Pin<&mut Self>, cx: &mut std::task::Context<'_>) -> std::task::Poll<Self::Output> {
        let mut lock = self.inner.lock().unwrap();
        let mutref = lock.as_mut();
        mutref.poll(cx)
    }
}