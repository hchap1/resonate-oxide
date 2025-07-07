use std::mem::replace;
use std::task::Waker;
use std::task::Context;
use std::task::Poll;
use std::pin::Pin;
use std::thread::spawn;
use std::thread::JoinHandle;
use crossbeam_channel::Sender;
use crossbeam_channel::Receiver;
use crossbeam_channel::unbounded;
use iced::futures::Stream;

pub fn consume(string: &mut String) -> String {
    replace(string, String::new())
}

#[pin_project::pin_project]
pub struct Relay<T, F, M>
where
        F: Fn(T) -> Option<M>
{
    waker_sender: Sender<Waker>,
    queue_receiver: Receiver<T>,
    _handle: JoinHandle<()>,
    map_fn: F,
    packets: Vec<T>
}

impl<T: Send + 'static + std::fmt::Debug, F: Fn(T) -> Option<M>, M> Relay<T, F, M> {
    pub fn consume_receiver(receiver: Receiver<T>, map_fn: F) -> Relay<T, F, M> {
        let (waker_sender, waker_receiver) = unbounded();
        let (queue_sender, queue_receiver) = unbounded();

        Relay::<T, F, M> {
            waker_sender,
            queue_receiver,
            _handle: spawn(move || relay(waker_receiver, queue_sender, receiver)),
            map_fn,
            packets: Vec::new()
        }
    }
}

fn relay<T: std::fmt::Debug>(
    waker_receiver: Receiver<Waker>,
    queue_sender: Sender<T>,
    receiver: Receiver<T>
) {
    let mut waker = match waker_receiver.recv() {
        Ok(waker) => waker,
        Err(_) => return,
    };

    loop {
        // Attempt to receive packets.
        let packet = match receiver.recv() {
            Ok(packet) => packet,
            Err(_) => break
        };

        if queue_sender.send(packet).is_err() { break; }
        waker.wake_by_ref();


        waker = match waker_receiver.recv() {
            Ok(waker) => waker,
            Err(_) => break
        };
    }

    waker.wake_by_ref();
    while !queue_sender.is_empty() {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

impl<T: std::fmt::Debug, F: Fn(T) -> Option<M>, M> Stream for Relay<T, F, M> {
    type Item = M;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<M>> {
        let waker = context.waker().to_owned();
        if self.waker_sender.send(waker).is_err() {
            return Poll::Ready(None);
        }

        let packet = self.queue_receiver.try_recv().ok();

        match packet {
            Some(packet) => return Poll::Ready(
                (self.map_fn)(packet)
            ),
            None => match self._handle.is_finished() {
                true => Poll::Ready(None),
                false => Poll::Pending
            }
        }
    }
}
