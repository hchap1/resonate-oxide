use std::path::PathBuf;
use std::time::Duration;

use std::process::Stdio;

use std::pin::Pin;
use std::task::Poll;
use std::task::Context;
use std::task::Waker;

use iced::futures::Stream;
use iced::futures::StreamExt;
use rspotify::model::FullTrack;
use rspotify::model::Id;
use rspotify::model::PlaylistId;
use rspotify::model::PlaylistItem;
use rspotify::clients::BaseClient;
use rspotify::ClientCredsSpotify;

use tokio::task::JoinHandle;
use tokio::task::spawn;
use tokio::process::Command;
use tokio::io::BufReader;
use tokio::io::AsyncBufReadExt;

use crossbeam_channel::Receiver;
use crossbeam_channel::Sender;
use crossbeam_channel::unbounded;

use crate::frontend::message::Message;

use super::music::Song;
use super::database::Database;
use super::util::AM;
use super::util::desync;

pub async fn try_auth(credentials: ClientCredsSpotify) -> Result<ClientCredsSpotify, ()> {
    match credentials.request_token().await {
        Ok(_) => Ok(credentials),
        Err(_) => Err(())
    }
}

pub struct SpotifySongStream {
    handle: JoinHandle<Result<(), ()>>,
    sender: Sender<InterThreadMessage>,
    receiver: Receiver<InterThreadMessage>,
    waker_received: bool
}

impl SpotifySongStream {
    pub fn new(
        playlist_link: String,
        credentials: ClientCredsSpotify,

    ) -> SpotifySongStream {

        let (sender, thread_receiver) = unbounded();
        let (thread_sender, receiver) = unbounded();

        let handle = spawn(consume_stream(
            playlist_link,
            credentials,
            thread_receiver,
            thread_sender
        ));

        SpotifySongStream {
            handle,
            sender,
            receiver,
            waker_received: false
        }
    }
}

enum InterThreadMessage {
    Done,
    Result(PlaylistItem),
    Waker(Waker),
    WakerReceived
}

async fn consume_stream(
    playlist_link: String,
    credentials: ClientCredsSpotify,
    receiver: Receiver<InterThreadMessage>,
    sender: Sender<InterThreadMessage>
) -> Result<(), ()> {
    let playlist_id = match PlaylistId::from_uri(playlist_link.as_str()) {
        Ok(playlist_id) => match PlaylistId::from_id(playlist_id.id().to_owned()) {
            Ok(playlist_id) => playlist_id,
            Err(_) => return Err(())
        }
        Err(_) => return Err(())
    };

    let mut stream = credentials.playlist_items(playlist_id, None, None);
    let duration = Duration::from_millis(100);

    let waker = 'outer: loop {
        tokio::time::sleep(duration);

        while let Ok(message) = receiver.try_recv() {
            match message {
                InterThreadMessage::Done => return Err(()), // shouldnt happen
                InterThreadMessage::Waker(waker) => break 'outer waker,
                _ => continue
            }
        }
    };

    match sender.send(InterThreadMessage::WakerReceived) {
        Ok(_) => {},
        Err(_) => {
            waker.wake_by_ref();
            return Err(())
        }
    }

    loop {
        match stream.next().await {
            Some(playlist_item) => match playlist_item {
                Ok(playlist_item) => match sender.send(InterThreadMessage::Result(playlist_item)) {
                    Ok(_) => {},
                    Err(_) => {
                        waker.wake_by_ref();
                        return Err(())
                    }
                },
                Err(_) => {
                    let _ = sender.send(InterThreadMessage::Done);
                    waker.wake_by_ref();
                    return Err(())
                }
            }   
            None => {
                let _ = sender.send(InterThreadMessage::Done);
                waker.wake_by_ref();
                return Err(())
            }
        }

        waker.wake_by_ref();
    }
}

impl Stream for SpotifySongStream {
    type Item = PlaylistItem;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<PlaylistItem>> {
        if !self.waker_received {
            self.sender.send(InterThreadMessage::Waker(context.waker().to_owned()));
        }

        if self.handle.is_finished() {
            return Poll::Ready(None); // done
        }

        match self.receiver.try_recv() {
            Ok(message) => match message {
                InterThreadMessage::Done => return Poll::Ready(None), // done
                InterThreadMessage::Result(res) => return Poll::Ready(Some(res)),
                InterThreadMessage::WakerReceived => self.waker_received = true,
                _ => {}
            },
            Err(crossbeam_channel::TryRecvError::Disconnected) => return Poll::Ready(None),
            Err(crossbeam_channel::TryRecvError::Empty) => return Poll::Pending
        };

        Poll::Pending
    }
}

pub async fn load_spotify_song(
    item: FullTrack,
    dlp_path: PathBuf,
    database: AM<Database>,
    music_path: PathBuf,
    thumbnail_path: PathBuf
) -> Result<Song, ()> {

    let artist = item.artists.iter().map(|artist| artist.name).collect::<Vec<String>>().join(" ");

    let search = format!("\"ytsearch1:{} {}\"", item.name, artist);

    let process = Command::new(dlp_path)
        .arg(search)
        .arg("--print")
        .arg("\"id\"")
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|_| ())?;

    let stdout = process.stdout.ok_or(())?;
    let reader = BufReader::new(stdout);

    let id = match reader.lines().next_line().await {
        Ok(id) => match id {
            Some(id) => id,
            None => return Err(())
        },
        Err(_) => return Err(())
    };

    if id.len() != 11 {
        return Err(());
    }

    let mut base_song = Song::load(0, id,
        item.name,
        artist,
        Some(item.album.name),
        item.duration.to_std().unwrap_or(Duration::from_secs(0)),
        &music_path,
        &thumbnail_path
    );

    let (_, id) = match desync(&database).emplace_song_and_record_id(&base_song, true) {
        Ok(data) => data,
        Err(_) => return Err(())
    };

    base_song.id = id;
    Ok(base_song)
}
