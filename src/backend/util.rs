use std::mem::replace;
use std::task::Waker;
use std::task::Context;
use std::task::Poll;
use std::pin::Pin;
use std::thread::spawn;
use std::thread::JoinHandle;
use async_channel::Sender;
use async_channel::Receiver;
use async_channel::unbounded;
use iced::futures::Stream;
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

use super::music::Song;

pub fn consume(string: &mut String) -> String {
    replace(string, String::new())
}

#[pin_project::pin_project]
pub struct Relay<T: std::fmt::Debug, F, M>
where
        F: Fn(T) -> Option<M>
{
    waker_sender: Sender<Waker>,
    queue_receiver: Receiver<T>,
    _handle: JoinHandle<()>,
    map_fn: F,
    packets: Vec<T>,
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
            packets: Vec::new(),
        }
    }
}

fn relay<T: std::fmt::Debug>(
    waker_receiver: Receiver<Waker>,
    queue_sender: Sender<T>,
    receiver: Receiver<T>,
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
        waker.wake();

        waker = match waker_receiver.recv() {
            Ok(waker) => waker,
            Err(_) => break
        };
    }

    waker = match waker_receiver.recv() {
        Ok(waker) => waker,
        Err(_) => return
    };

    waker.wake();
    while !queue_sender.is_empty() {
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

impl<T: std::fmt::Debug, F: Fn(T) -> Option<M>, M> Stream for Relay<T, F, M> {
    type Item = M;

    fn poll_next(self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<M>> {
        let packet = self.queue_receiver.try_recv().ok();

        match packet {
            Some(packet) => return Poll::Ready(
                (self.map_fn)(packet)
            ),
            None => match self._handle.is_finished() {
                true => Poll::Ready(None),
                false => {
                    let waker = context.waker().to_owned();
                    if self.waker_sender.send(waker).is_err() {
                        return Poll::Ready(None);
                    }

                    Poll::Pending
                }
            }
        }
    }
}

pub fn is_song_similar(song: &Song, query: &String) -> usize {
    let matcher = SkimMatcherV2::default();
    let mut title_score = matcher.fuzzy_match(&song.title, query).unwrap_or(0);
    let mut artist_score = matcher.fuzzy_match(&song.artist, query).unwrap_or(0);
    let mut album_score = song.album.as_ref().map_or(Some(0), |a| matcher.fuzzy_match(a, query)).unwrap_or(0);

    if title_score < 0 { title_score = 0; }
    if artist_score < 0 { artist_score = 0; }
    if album_score < 0 { album_score = 0; }

    (title_score + artist_score + album_score) as usize
}
