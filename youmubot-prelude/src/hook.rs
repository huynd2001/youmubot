use crate::{async_trait, future, Context, Result};
use serenity::model::channel::Message;

/// Hook represents the asynchronous hook that is run on every message.
#[async_trait]
pub trait Hook: Send + Sync {
    async fn call(&mut self, ctx: &Context, message: &Message) -> Result<()>;
}

#[async_trait]
impl<T> Hook for T
where
    T: for<'a> FnMut(
            &'a Context,
            &'a Message,
        )
            -> std::pin::Pin<Box<dyn future::Future<Output = Result<()>> + 'a + Send>>
        + Send
        + Sync,
{
    async fn call(&mut self, ctx: &Context, message: &Message) -> Result<()> {
        self(ctx, message).await
    }
}
