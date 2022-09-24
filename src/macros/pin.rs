// heavily borrowed from Tokio https://docs.rs/tokio/latest/src/tokio/macros/pin.rs.html#
/// Pins a value on the stack.
///
/// Calls to `async fn` return anonymous [`Future`] values that are `!Unpin`.
/// These values must be pinned before they can be polled. Calling `.await` will
/// handle this, but consumes the future. If it is required to call `.await` on
/// a `&mut _` reference, the caller is responsible for pinning the future.
///
/// Pinning may be done by allocating with [`Box::pin`] or by using the stack
/// with the `pin!` macro.
///
/// The following will **fail to compile**:
///
/// ```compile_fail
/// async fn my_async_fn() {
///     // async logic here
/// }
///
/// let mut future = my_async_fn();
/// (&mut future).await;

/// ```
///
/// To make this work requires pinning:
///
/// ```
/// use tokio::pin;
///
/// async fn my_async_fn() {
///     // async logic here
/// }
///

/// let future = my_async_fn();
/// pin!(future);
/// (&mut future).await;
/// ```
///
/// # Usage
///
/// The `pin!` macro takes **identifiers** as arguments. It does **not** work
/// with expressions.
///
/// The following does not compile as an expression is passed to `pin!`.
///
/// ```compile_fail
/// async fn my_async_fn() {
///     // async logic here
/// }
///
/// let mut future = pin!(my_async_fn());
/// (&mut future).await;
/// ```
///
#[macro_export]
macro_rules! pin {
    ($($x:ident),*) => { $(
        // Move the value to ensure that it is owned
        let mut $x = $x;
        // Shadow the original binding so that it can't be directly accessed
        // ever again.
        #[allow(unused_mut)]
        let mut $x = unsafe {
            core::pin::Pin::new_unchecked(&mut $x)
        };

    )* };
}
