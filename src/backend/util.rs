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
use crossbeam_channel::Sender;
use crossbeam_channel::Receiver;
use crossbeam_channel::unbounded;
use iced::futures::Stream;

pub type AM<T> = Arc<Mutex<T>>;

pub fn sync<T>(obj: T) -> AM<T> { Arc::new(Mutex::new(obj)) }
pub fn desync<T>(obj: &AM<T>) -> MutexGuard<T> { obj.lock().unwrap() }

pub fn consume(string: &mut String) -> String {
    replace(string, String::new())
}

#[derive(Debug)]
enum RelayPacket<T: std::fmt::Debug> {
    Data(T),
    Handshake
}

#[pin_project::pin_project]
pub struct Relay<T: std::fmt::Debug, F, M>
where
        F: Fn(T) -> M,
{
    waker_confirmed: bool,
    waker_sender: Sender<Waker>,
    queue_receiver: Receiver<RelayPacket<T>>,
    _handle: JoinHandle<()>,
    map_fn: F,
    packets: Vec<T>,
}

impl<T: Send + 'static + std::fmt::Debug, F: Fn(T) -> M, M> Relay<T, F, M> {
    pub fn consume_receiver(receiver: Receiver<T>, map_fn: F) -> Relay<T, F, M> {
        let (waker_sender, waker_receiver) = unbounded();
        let (queue_sender, queue_receiver) = unbounded();

        Relay::<T, F, M> {
            waker_confirmed: false,
            waker_sender,
            queue_receiver,
            _handle: spawn(move || relay(waker_receiver, queue_sender, receiver)),
            map_fn,
            packets: Vec::new(),
        }
    }
}

fn relay<T: std::fmt::Debug>(
    waker_receiver: Receiver<Waker>, queue_sender: Sender<RelayPacket<T>>, receiver: Receiver<T>
) {

    let mut waker = loop {
        match waker_receiver.recv() {
            Ok(waker) => break waker,
            Err(_) => return
        }
    };

    if queue_sender.send(RelayPacket::Handshake).is_err() { return; }

    loop {
        let packet = match receiver.recv() {
            Ok(packet) => packet,
            Err(_) => return
        };

        if queue_sender.send(RelayPacket::Data(packet)).is_err() { return; }

        while let Ok(new_waker) = waker_receiver.try_recv() {
            waker = new_waker;
        }

        waker.wake_by_ref();
    }
}

impl<T: std::fmt::Debug, F: Fn(T) -> M, M> Stream for Relay<T, F, M> {
    type Item = M;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<M>> {
        let waker = context.waker().to_owned();
        if self.waker_sender.send(waker).is_err() {
            return Poll::Ready(None);
        }

        while let Ok(packet) = self.queue_receiver.try_recv() {
            match packet {
                RelayPacket::Data(data) => self.packets.push(data),
                RelayPacket::Handshake => self.waker_confirmed = true
            }
        }

        match self.packets.pop() {
            Some(packet) => return Poll::Ready(Some(
                (self.map_fn)(packet)
            )),
            None => match self._handle.is_finished() {
                true => Poll::Ready(None),
                false => Poll::Pending
            }
        }
    }
}
