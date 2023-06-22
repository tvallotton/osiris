use std::{
    cell::RefCell,
    collections::VecDeque,
    error::Error,
    fmt::{Debug, Display, Formatter},
    future::poll_fn,
    rc::Rc,
    task::{Poll, Waker},
};

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
#[derive(PartialEq, Eq, Clone, Copy)]
pub struct SendError<T>(pub T);

#[derive(PartialEq, Eq, Clone, Copy, Debug)]
pub struct RecvError;

pub enum Queue<T> {
    Rendezvous(Option<T>),
    Bounded(VecDeque<T>),
}

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
    ///
    pub async fn send(&self, item: T) -> Result<(), SendError<T>> {
        let mut item = Some(item);
        let mut waker_guard = None;
        poll_fn(|cx| {
            let mut ch = self.channel().borrow_mut();
            if ch.receivers == 0 && item.is_none() {
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
                let Some(value) = value.take() else { unreachable!() };
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
fn mpmc_stress_test() {
    crate::block_on(async {
        const N: usize = 1_000;
        let (s, r) = channel(2);

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
            if fastrand::i8(..) == 0 {
                println!("1");
            }
            send.await;
            recv.await;
        }
    })
    .unwrap();
}
