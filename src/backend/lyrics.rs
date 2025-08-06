use std::thread::JoinHandle;

use async_channel::Receiver;
use async_channel::Sender;
use async_channel::unbounded;

use chartlyrics::BlockingClient;
type Client = BlockingClient;

use super::{error::ResonateError, music::Song};

pub struct Lyrics {
    _handle: JoinHandle<()>,
    sender: Sender<Song>,
    receiver: Option<Receiver<String>>
}

impl Lyrics {
    pub fn new() -> Option<Self> {
        Client::new().ok().map(|client| {
            let (thread_sender, my_receiver) = unbounded();
            let (my_sender, thread_receiver) = unbounded();
            Self { 
                _handle: std::thread::spawn(move || Self::run(client, thread_sender, thread_receiver)),
                sender: my_sender,
                receiver: Some(my_receiver)
            }
        })
    }

    pub fn get_lyrics(client: &Client, song: Song) -> Result<String, ResonateError> {
        match client.search_lyric_direct(&song.title, &song.artist) {
            Ok(lyrics) => Ok(lyrics.lyrics),
            Err(_) => Err(ResonateError::NetworkError)
        }
    }

    pub fn run(client: Client, sender: Sender<String>, receiver: Receiver<Song>) {
        while let Ok(target_song) = receiver.recv_blocking() {
            if let Ok(result) = Self::get_lyrics(&client, target_song) {
                let _ = sender.send_blocking(result);
            }
        }
    }

    pub fn take_receiver(&mut self) -> Option<Receiver<String>> {
        self.receiver.take()
    }

    pub fn send(&self, song: Song) {
        let _ = self.sender.send_blocking(song);
    }
}
