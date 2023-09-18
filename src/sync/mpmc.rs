//! Multi-producer, multi-consumer FIFO queue communication primitives.
//!
//! Note that unlike std's, tokio's, or crossbeam's channels, osiris's channel
//! is designed to be used across tasks, not across threads, so they do not implement
//! the `Send` and `Sync` traits. Synchronizing tasks is cheaper than
//! synchronizing threads, so when working with osiris tasks, this implementation is a
//! good choice.
//!
//! This module provides message-based communication over channels, concretely
//! defined among two types:
//!
//! * [`Sender`]
//! * [`Receiver`]
//!
//! A [`Sender`] is used to send data to a [`Receiver`]. Both
//! types are clone-able (multi-producer and multi-consumer) such that many tasks can send
//! simultaneously and multiple tasks can receive from the channel.
//!
//! ## Disconnection
//!
//! The send and receive operations on channels will all return a [`Result`]
//! indicating whether the operation succeeded or not. An unsuccessful operation
//! is normally indicative of the other half of a channel having "hung up" by
//! being dropped in its corresponding thread.
//!
//! Once half of a channel has been deallocated, most operations can no longer
//! continue to make progress, so [`Err`] will be returned. Many applications
//! will continue to [`unwrap`] the results returned from this module,
//! instigating a propagation of failure among threads if one unexpectedly dies.
//!
//! ## Rendezvous channels
//! Rendezvous channels have no buffer. The sending half of the channel
//! suspends until the consuming task invokes receive on the channel. In order
//! to create rendezvous channels a buffer capacity of zero must be specified.
//!
//! [`unwrap`]: Result::unwrap
//!
//! # Examples
//!
//! Simple usage:
//!
//! ```
//! use osiris::sync::mpmc::channel;
//! use osiris::detach;
//!
//! #[osiris::main]
//! async fn main() {
//!     // Create a simple streaming channel
//!     let (tx, rx) = channel(2);
//!
//!     detach(async move {
//!         tx.send(10).await.unwrap();
//!     });
//!     assert_eq!(rx.recv().await.unwrap(), 10);
//! }
//! ```
//!
//! Shared usage:
//!
//! ```
//! use osiris::detach;
//! use osiris::sync::mpmc::channel;
//!
//! #[osiris::main]
//! async fn main() {
//!     // Create a shared channel that can be sent along from many threads
//!     // where tx is the sending half (tx for transmission), and rx is the receiving
//!     // half (rx for receiving).
//!     let (tx, rx) = channel(8);
//!     for i in 0..10 {
//!         let tx = tx.clone();
//!         detach(async move {
//!             tx.send(i).await.unwrap();
//!         });
//!     }
//!    
//!     for _ in 0..10 {
//!         let j = rx.recv().await.unwrap();
//!         assert!(0 <= j && j < 10);
//!     }
//! }
//! ```
//!
//! Propagating panics:
//!
//! ```
//! use osiris::sync::mpmc::channel;
//!
//! #[osiris::main]
//! async fn main() {
//!     // The call to recv() will return an error because the channel has already
//!     // been closed
//!     let (tx, rx) = channel::<i32>(1);
//!     drop(tx);
//!     assert!(rx.recv().await.is_err());
//! }
//! ```
//!
//! Rendezvous channels:
//!
//! ```
//! use osiris::sync::mpmc::channel;
//! use osiris::time::{sleep, Duration};
//! use osiris::spawn;
//!
//! #[osiris::main]
//! async fn main() {
//!     // a capacity of zero makes the channel rendezvous
//!     let (tx, rx) = channel::<i32>(0);
//!     
//!     let handle = spawn(async move {
//!         // the sender will wait for the receiver
//!         // to receive the element
//!         // efectively waiting for one second
//!         tx.send(1).await;
//!     });
//!     
//!     sleep(Duration::from_secs(1)).await;
//!     rx.recv().await;
//! }
//! ```

use std::cell::RefCell;
use std::collections::VecDeque;
use std::error::Error;
use std::fmt::{Debug, Display, Formatter};
use std::future::poll_fn;
use std::rc::Rc;
use std::task::{Poll, Waker};

/// The sending-half of osiris's asynchronous [`channel`] type.
///
/// Messages can be sent through this channel with [`send`].
///
/// Note: all senders (the original and the clones) need to be dropped for the receiver
/// to stop waiting to receive messages with [`Receiver::recv`].
///
/// [`send`]: Sender::send
///
/// # Examples
///
/// ```rust
/// use osiris::sync::mpmc::channel;
/// use osiris::detach;
///
/// #[osiris::main]
/// async fn main() {
///     let (sender, receiver) = channel(1);
///     let sender2 = sender.clone();
///     
///     detach(async move {
///         sender.send(1).await.unwrap();
///     });
///    
///     // Second thread owns sender2
///     detach(async move {
///         sender2.send(2).await.unwrap();
///     });
///     
///     let msg = receiver.recv().await.unwrap();
///     let msg2 = receiver.recv().await.unwrap();
///     assert_eq!(3, msg + msg2);
/// }
/// ```
pub struct Sender<T>(Rc<RefCell<Channel<T>>>);

/// The receiving half of osiris's [`channel`] type.
///
/// Messages sent to the channel can be retrieved using [`recv`].
///
/// [`recv`]: Receiver::recv
///
/// # Examples
///
/// ```rust
/// use osiris::sync::mpmc::channel;
/// use osiris::time::{Duration, sleep};
/// use osiris::spawn;
///
/// #[osiris::main]
///     async fn main() {
///     let (send, recv) = channel(1);
///     
///     let handle = spawn(async move {
///         send.send("Hello world!").await.unwrap();
///         sleep(Duration::from_secs(1)).await; // sleep for two seconds
///         send.send("Delayed for 1 seconds").await.unwrap();
///     });
///     
///     assert_eq!("Hello world!", recv.recv().await.unwrap()); // Received immediately
///     println!("Waiting...");
///     assert_eq!("Delayed for 1 seconds", recv.recv().await.unwrap()); // Received after 2 seconds
/// }
/// ```
pub struct Receiver<T>(Rc<RefCell<Channel<T>>>);

struct Channel<T> {
    /// reference count for the number of senders
    senders: u32,
    /// reference count for the number of receivers
    receivers: u32,
    /// queue of items to be sent
    queue: Queue<T>,
    sender_id: u32,
    receiver_id: u32,
    send_wakers: VecDeque<(u32, Waker)>,
    recv_waiters: VecDeque<(u32, Waker)>,
}
/// An error returned from the [`Sender::send`]
/// function on **channel**s.
///
/// A **send** operation can only fail if the receiving end of a channel is
/// disconnected, implying that the data could never be received. The error
/// contains the data being sent as a payload so it can be recovered
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct SendError<T>(pub T);

/// An error returned from the [`recv`] function on a [`Receiver`].
///
/// The [`recv`] operation can only fail if the sending half of a
/// [`channel`] is disconnected, implying that no further
/// messages will ever be received.
///
/// [`recv`]: Receiver::recv
#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct RecvError;

enum Queue<T> {
    Rendezvous(Option<T>),
    Bounded(VecDeque<T>),
}

/// Creates a bounded mpmc channel for communicating between asynchronous tasks
/// with backpressure.
///
/// The channel will buffer up to the provided number of messages.  Once the
/// buffer is full, attempts to send new messages will wait until a message is
/// received from the channel.
///
/// All data sent on `Sender` will become available on `Receiver` in the same
/// order as it was sent.
///
/// The `Sender` and `Receiver` types can be cloned to `send` and `recv` items from the same channel
/// from multiple code locations.
///
/// If the `Receiver` is disconnected while trying to `send`, the `send` method
/// will return a `SendError`. Similarly, if `Sender` is disconnected while
/// trying to `recv`, the `recv` method will return `RecvError`.
///
/// A capacity of zero is supported, and it will make the channel behave like a
/// *rendezvous* channel.
///
///
/// # Examples
///
/// ```rust
/// use osiris::sync::mpmc::channel;
/// use osiris::spawn;
///
/// #[osiris::main]
/// async fn main() {
///     let (tx, rx) = channel(100);
///
///     let handle = spawn(async move {
///         for i in 0..10 {
///             tx.send(i).await.ok();
///         }
///     });
///
///     for i in 0..10 {
///         let Ok(msg) = rx.recv().await else { unreachable!() };
///         assert_eq!(i, msg);
///     }
///
///     handle.await;
/// }
/// ```
pub fn channel<T>(bound: usize) -> (Sender<T>, Receiver<T>) {
    let queue = if bound == 0 {
        Queue::Rendezvous(None)
    } else {
        Queue::Bounded(VecDeque::with_capacity(bound))
    };
    let channel = Channel {
        senders: 1,
        receivers: 1,
        sender_id: 0,
        receiver_id: 0,
        send_wakers: VecDeque::new(),
        recv_waiters: VecDeque::new(),
        queue,
    };

    let channel = Rc::new(RefCell::new(channel));
    (Sender(channel.clone()), Receiver(channel))
}

impl<T> Sender<T> {
    /// Attempts to send a value on this channel, returning it back if it could
    /// not be sent.
    ///
    /// A successful send occurs when it is determined that the other end of
    /// the channel has not hung up already. An unsuccessful send would be one
    /// where the corresponding receiver has already been deallocated. Note
    /// that for non-rendezvous channels, a return value of [`Err`] means that
    /// the data will never be received, but a return value of [`Ok`] does *not*
    /// mean that the data will be received. It is possible for the corresponding
    /// receiver to hang up immediately after this function returns [`Ok`]. Rendezvous
    /// channels wait until the receiver takes the value, thus guaranteeing the message
    /// was received with [`Ok`], which is achieved at the expense of lower throughput.
    ///
    /// This method will never block the current thread.
    ///
    /// # Examples
    ///
    /// ```
    /// use osiris::sync::mpmc::channel;
    ///
    /// #[osiris::main]
    /// async fn main() {
    ///     let (tx, rx) = channel(1); // non-rendezvous
    ///
    ///     // This send is always successful
    ///     tx.send(1).await.unwrap();
    ///
    ///     drop(rx);
    ///
    ///     // This send will fail because the receiver is gone
    ///     assert_eq!(tx.send(1).await.unwrap_err().0, 1);
    /// }
    /// ```
    pub async fn send(&self, item: T) -> Result<(), SendError<T>> {
        let mut item = Some(item);
        let mut waker_guard = None;
        poll_fn(|cx| {
            let mut ch = self.channel().borrow_mut();
            if ch.receivers == 0 && item.is_some() {
                // no receivers, returning error
                let item = item.take().unwrap();
                return Poll::Ready(Err(SendError(item)));
            }

            // if there is a queue, we put ourselves at the end
            if !ch.send_wakers.is_empty() && waker_guard.is_none() {
                drop(ch);
                waker_guard = Some(self.push_sender(cx.waker().clone()));
                return Poll::Pending;
            }

            if item.is_some() {
                // trying to send item
                let Ok(_) = ch.queue.try_push(&mut item) else {
                    // reached max capacity, waiting
                    drop(ch);
                    waker_guard = Some(self.push_sender(cx.waker().clone()));
                    return Poll::Pending;
                };
                // notify receiver that we've pushed
                if let Some((_, waker)) = ch.recv_waiters.pop_back() {
                    waker.wake();
                }
            }
            // if it is rendezvous we wait for the receiver to consume
            if matches!(ch.queue, Queue::Rendezvous(Some(_))) {
                drop(ch);
                waker_guard = Some(self.push_sender(cx.waker().clone()));
                return Poll::Pending;
            }

            Poll::Ready(Ok(()))
        })
        .await
    }

    fn push_sender(&self, waker: Waker) -> impl Drop + '_ {
        struct Guard<'a, T> {
            sender: &'a Sender<T>,
            id: u32,
        }
        #[allow(clippy::option_map_unit_fn)]
        impl<'a, T> Drop for Guard<'a, T> {
            fn drop(&mut self) {
                let mut channel = self.sender.channel().borrow_mut();
                channel
                    .send_wakers
                    .iter()
                    .position(|(id, _)| *id == self.id)
                    .map(|index| channel.send_wakers.remove(index));
                // we wake the next sender
                channel
                    .send_wakers
                    .pop_front()
                    .map(|(_, waker)| waker.wake());
            }
        }

        let mut ch = self.channel().borrow_mut();
        let sender_id = ch.sender_id();
        ch.send_wakers.push_back((sender_id, waker));
        Guard {
            id: sender_id,
            sender: self,
        }
    }
}

impl<T> Receiver<T> {
    /// Attempts to wait for a value on this receiver, returning an error if the
    /// corresponding channel has hung up.
    ///
    /// This function will always wait if there is no data available and it's possible
    ///  for more data to be sent (at least one sender still exists). Once a message is
    /// sent to the corresponding [`Sender`] this receiver will wake
    ///  up and return that message.
    ///
    /// If the corresponding [`Sender`] has disconnected, or it disconnects while
    /// this call is waiting, this call will wake up and return [`Err`] to
    /// indicate that no more messages can ever be received on this channel.
    /// However, since channels are buffered, messages sent before the disconnect
    /// will still be properly received.
    ///
    /// # Examples
    ///
    /// ```
    /// use osiris::sync::mpmc;
    ///
    /// #[osiris::main]
    /// async fn main() {
    ///     let (send, recv) = mpmc::channel(1);
    ///
    ///     let handle = osiris::spawn(async move {
    ///         send.send(1u8).await.unwrap();
    ///     });
    ///
    ///     handle.await;
    ///
    ///     assert_eq!(Ok(1), recv.recv().await);
    /// }
    /// ```
    ///
    /// Buffering behavior:
    ///
    /// ```
    /// use osiris::sync::mpmc::{RecvError, channel};
    /// use osiris::spawn;
    ///
    /// #[osiris::main]
    /// async fn main() {
    ///     let (send, recv) = channel(8);
    ///     let handle = spawn(async move {
    ///         send.send(1u8).await.unwrap();
    ///         send.send(2).await.unwrap();
    ///         send.send(3).await.unwrap();
    ///         drop(send);
    ///     });
    ///    
    ///     // wait for the thread to join so we ensure the sender is dropped
    ///     handle.await;
    ///     
    ///     // we receive even though there are no senders
    ///     assert_eq!(Ok(1), recv.recv().await);
    ///     assert_eq!(Ok(2), recv.recv().await);
    ///     assert_eq!(Ok(3), recv.recv().await);
    ///     // now that there is nothing else to receive we error
    ///     assert_eq!(Err(RecvError), recv.recv().await);
    /// }
    /// ```
    /// Receive with timeout:
    ///
    /// ```
    /// use osiris::sync::mpmc::{RecvError, channel};
    /// use osiris::spawn;
    /// use osiris::time::{timeout, Duration};
    ///
    /// #[osiris::main]
    /// async fn main() {
    ///     let (send, recv) = channel::<()>(8);
    ///     
    ///     let time = Duration::from_millis(100);
    ///     let res = timeout(time, recv.recv()).await;
    ///     assert_eq!(Err(timeout::Error), res);
    /// }
    /// ```
    ///
    pub async fn recv(&self) -> Result<T, RecvError> {
        let mut waker_guard = None;
        poll_fn(|cx| {
            let mut ch = self.channel().borrow_mut();
            let Some(item) = ch.queue.pop_front() else {
                // no items in the queue
                if ch.senders == 0 {
                    //  no senders, returning error
                    return Poll::Ready(Err(RecvError));
                }
                // register waker and wait
                drop(ch);
                let waker = cx.waker().clone();
                waker_guard = Some(self.push_receiver(waker));
                return Poll::Pending;
            };

            if let Some((_, waker)) = ch.send_wakers.pop_back() {
                waker.wake();
            }

            Poll::Ready(Ok(item))
        })
        .await
    }

    fn push_receiver(&self, waker: Waker) -> impl Drop + '_ {
        struct Guard<'a, T> {
            receiver: &'a Receiver<T>,
            id: u32,
        }
        #[allow(warnings)]
        impl<'a, T> Drop for Guard<'a, T> {
            fn drop(&mut self) {
                let mut channel = self.receiver.channel().borrow_mut();
                channel
                    .recv_waiters
                    .iter()
                    .position(|(id, _)| *id == self.id)
                    .map(|index| channel.recv_waiters.remove(index));
                // we wake the next sender
                channel
                    .recv_waiters
                    .pop_front()
                    .map(|(_, waker)| waker.wake());
            }
        }

        let mut ch = self.channel().borrow_mut();
        let receiver_id = ch.receiver_id();
        ch.recv_waiters.push_back((receiver_id, waker));
        Guard {
            id: receiver_id,
            receiver: self,
        }
    }
}

impl<T> Queue<T> {
    fn try_push(&mut self, value: &mut Option<T>) -> Result<(), ()> {
        match self {
            Queue::Bounded(queue) if queue.len() < queue.capacity() => {
                let Some(value) = value.take() else {
                    unreachable!()
                };
                queue.push_back(value);
                Ok(())
            }
            Queue::Rendezvous(option) if option.is_none() => {
                *option = value.take();
                Ok(())
            }
            _ => Err(()),
        }
    }

    fn pop_front(&mut self) -> Option<T> {
        match self {
            Queue::Bounded(queue) => queue.pop_front(),
            Queue::Rendezvous(option) => option.take(),
        }
    }

    fn is_rendezvous(&mut self) -> bool {
        matches!(self, Queue::Rendezvous(_))
    }

    fn is_some(&mut self) -> bool {
        matches!(self, Queue::Rendezvous(Some(_)))
    }
}

impl<T> Channel<T> {
    fn sender_id(&mut self) -> u32 {
        self.sender_id += 1;
        self.sender_id
    }

    fn receiver_id(&mut self) -> u32 {
        self.receiver_id += 1;
        self.receiver_id
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let mut ch = self.channel().borrow_mut();
        ch.senders -= 1;
    }
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        let mut ch = self.channel().borrow_mut();
        ch.receivers -= 1;
    }
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        let mut ch = self.channel().borrow_mut();
        ch.senders += 1;
        Self(self.0.clone())
    }
}

impl<T> Clone for Receiver<T> {
    fn clone(&self) -> Self {
        let mut ch = self.channel().borrow_mut();
        ch.receivers += 1;
        Self(self.0.clone())
    }
}

impl<T> Sender<T> {
    fn channel(&self) -> &RefCell<Channel<T>> {
        &self.0
    }
}

impl<T> Receiver<T> {
    fn channel(&self) -> &RefCell<Channel<T>> {
        &self.0
    }
}

impl<T> Debug for SendError<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "SendError")
    }
}

impl<T> Display for SendError<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "sending on a closed channel")
    }
}

impl Display for RecvError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "receiving on a closed channel")
    }
}

impl Error for RecvError {}
impl<T> Error for SendError<T> {}

#[test]
fn mpmc_stress_test_rendezvous() {
    crate::block_on(async {
        const N: usize = 500;
        let (s, r) = channel(0);

        let mut tasks = vec![];
        for _ in 0..N {
            let s = s.clone();
            let r = r.clone();
            let send = crate::spawn(async move {
                while fastrand::f32() < 0.5 {
                    crate::task::yield_now().await;
                }
                s.send(1).await.ok();
            });

            let recv = crate::spawn(async move {
                while fastrand::f32() < 0.5 {
                    crate::task::yield_now().await;
                }
                r.recv().await.ok();
            });
            tasks.push((send, recv));
        }
        drop(s);
        drop(r);

        for i in 0..N {
            // cancel a pair of tasks at random
            if fastrand::f32() < 0.01 && i < tasks.len() {
                tasks.remove(i);
                crate::task::yield_now().await;
            } else if i >= tasks.len() {
                break;
            }
        }
        for (send, recv) in tasks {
            send.await;
            recv.await;
        }
    })
    .unwrap();
}

#[test]
fn mpmc_stress_test_bound() {
    crate::block_on(async {
        const N: usize = 500;
        let (s, r) = channel(8);

        let mut tasks = vec![];
        for _ in 0..N {
            let s = s.clone();
            let r = r.clone();
            let send = crate::spawn(async move {
                while fastrand::f32() < 0.5 {
                    crate::task::yield_now().await;
                }
                s.send(1).await.ok();
            });

            let recv = crate::spawn(async move {
                while fastrand::f32() < 0.5 {
                    crate::task::yield_now().await;
                }
                r.recv().await.ok();
            });
            tasks.push((send, recv));
        }
        drop(s);
        drop(r);

        for i in 0..N {
            // cancel a pair of tasks at random
            if fastrand::f32() < 0.01 && i < tasks.len() {
                tasks.remove(i);
                crate::task::yield_now().await;
            } else if i >= tasks.len() {
                break;
            }
        }
        for (send, recv) in tasks {
            send.await;
            recv.await;
        }
    })
    .unwrap();
}

#[test]
fn mpsc_send_recv_errors() {
    crate::block_on(async {
        let (s, r) = channel::<i32>(0);
        drop(s);
        assert!(r.recv().await.is_err());
        let (s, r) = channel::<i32>(0);
        drop(r);
        assert!(s.send(0).await.is_err());
    })
    .unwrap();
}
