use std::sync::Arc;
use std::sync::Mutex;
use std::sync::MutexGuard;
use std::mem::replace;
use std::task::Waker;
use std::task::Context;
use std::task::Poll;
use std::pin::Pin;
use std::thread::spawn;
use std::thread::JoinHandle;
use crossbeam_channel::Receiver;
use iced::futures::Stream;

pub type AM<T> = Arc<Mutex<T>>;
pub type AMO<T> = Arc<Mutex<Option<T>>>;
pub type AMV<T> = Arc<Mutex<Vec<T>>>;

pub fn sync<T>(obj: T) -> AM<T> { Arc::new(Mutex::new(obj)) }
pub fn desync<T>(obj: &AM<T>) -> MutexGuard<T> { obj.lock().unwrap() }

pub fn consume(string: &mut String) -> String {
    replace(string, String::new())
}

pub struct Relay<T, F, M>
where
        F: Fn(T) -> M,
{
    waker: AMO<Waker>,
    queue: AMV<T>,
    _handle: JoinHandle<()>,
    map_fn: F
}

impl<T: Send + 'static, F: Fn(T) -> M, M> Relay<T, F, M> {
    pub fn consume_receiver(receiver: Receiver<T>, map_fn: F) -> Relay<T, F, M> {
        let waker = sync(None);
        let queue = sync(vec![]);
        
        let waker_clone = waker.clone();
        let queue_clone = queue.clone();

        Relay::<T, F, M> {
            waker,
            queue,
            _handle: spawn(move || relay(waker_clone, queue_clone, receiver)),
            map_fn
        }
    }
}

fn relay<T>(waker: AMO<Waker>, queue: AMV<T>, receiver: Receiver<T>) {
    loop {
        if let Ok(data) = receiver.recv() {
            desync(&queue).push(data);
            if let Some(waker) = desync(&waker).as_ref() { waker.wake_by_ref(); }
        }
    }
}

impl<T, F: Fn(T) -> M, M> Stream for Relay<T, F, M> {
    type Item = M;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<M>> {
        *desync(&self.waker) = Some(context.waker().to_owned());

        if let Some(val) = desync(&self.queue).pop() {
            Poll::Ready(Some(
                (self.map_fn)(val)
            ))
        } else {
            Poll::Pending
        }
    }
}
