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

        println!("[SPOTIFY] Stream created");

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
    WakerReceived,
    PlaylistName(String)
}

async fn consume_stream(
    playlist_link: String,
    credentials: ClientCredsSpotify,
    receiver: Receiver<InterThreadMessage>,
    sender: Sender<InterThreadMessage>
) -> Result<(), ()> {
    println!("[SPOTIFY] Stream consuming async thread spawned");
    let playlist_id = match PlaylistId::from_id_or_uri(playlist_link.as_str()) {
        Ok(playlist_id) => playlist_id,
        Err(e) => {
            println!("[SPOTIFY] Failed to parse URI: {e:?}");
            return Err(())
        }
    };

    println!("[SPOTIFY] Async thread has valid ID");

    let mut stream = credentials.playlist_items(
        playlist_id.clone(),
        None,
        Some(rspotify::model::Market::Country(rspotify::model::Country::UnitedStates)),
    );


    let duration = Duration::from_millis(100);

    let waker = 'outer: loop {
        println!("[SPOTIFY] Async thread polling for waker");
        let _ = tokio::time::sleep(duration);

        while let Ok(message) = receiver.try_recv() {
            match message {
                InterThreadMessage::Done => return Err(()), // shouldnt happen
                InterThreadMessage::Waker(waker) => {
                    println!("[SPOTIFY] Async thread has found waker");
                    break 'outer waker
                }
                _ => continue
            }
        }
    };

    println!("[SPOTIFY] Async thread starting to drain stream");

    match sender.send(InterThreadMessage::WakerReceived) {
        Ok(_) => {},
        Err(_) => {
            println!("[SPOTIFY] Failed to send message confirming waker received");
            waker.wake_by_ref();
            return Err(())
        }
    }

    let playlist = credentials.playlist(
        playlist_id,
        None,
        Some(rspotify::model::Market::Country(rspotify::model::Country::UnitedStates)),
    ).await;

    match playlist {
        Ok(playlist) => {
            let _ = sender.send(InterThreadMessage::PlaylistName(playlist.name));
        },
        Err(_) => {}
    }

    loop {
        match stream.next().await {
            Some(playlist_item) => match playlist_item {
                Ok(playlist_item) => match sender.send(InterThreadMessage::Result(playlist_item)) {
                    Ok(_) => {
                        println!("[SPOTIFY] Async thread, pushing a new result.");
                    },
                    Err(_) => {
                        println!("[SPOTIFY] Async thread exiting due to failure to send result");
                        waker.wake_by_ref();
                        return Err(())
                    }
                },
                Err(_) => {
                    let _ = sender.send(InterThreadMessage::Done);
                    println!("[SPOTIFY] Async thread exiting because Paginator stream failed to produce item");
                    waker.wake_by_ref();
                    return Err(())
                }
            }   
            None => {
                let _ = sender.send(InterThreadMessage::Done);
                println!("[SPOTIFY] Async thread exiting because Paginator stream failed");
                waker.wake_by_ref();
                return Err(())
            }
        }

        waker.wake_by_ref();
    }
}

pub enum SpotifyEmmision {
    PlaylistItem(PlaylistItem),
    PlaylistName(String)
}

impl Stream for SpotifySongStream {
    type Item = SpotifyEmmision;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<SpotifyEmmision>> {
        println!("[SPOTIFY] Polling: Polled");
        if !self.waker_received {
            let _ = self.sender.send(InterThreadMessage::Waker(context.waker().to_owned()));
            println!("[SPOTIFY] Sending WAKER");
        }

        loop {
            match self.receiver.try_recv() {
                Ok(message) => match message {
                    InterThreadMessage::Done => {
                        println!("[SPOTIFY] Polling finished from receiving done.");
                        return Poll::Ready(None) // done
                    }
                    InterThreadMessage::Result(res) => {
                        println!("[SPOTIFY] Result!");
                        return Poll::Ready(Some(SpotifyEmmision::PlaylistItem(res)))
                    },
                    InterThreadMessage::WakerReceived => {
                        println!("[IMPORTANT] [SPOTIFY] Waker received and acknowledged");
                        self.waker_received = true;
                    }
                    InterThreadMessage::PlaylistName(name) => {
                        return Poll::Ready(Some(SpotifyEmmision::PlaylistName(name)))
                    }
                    _ => {
                        println!("[SPOTIFY] Received useless message.");
                    }
                },
                Err(crossbeam_channel::TryRecvError::Disconnected) => return Poll::Ready(None),
                _ => {
                    break;
                }
            };
        }

        if self.handle.is_finished() {
            println!("[SPOTIFY] Handle is finished, thus ending stream");
            Poll::Ready(None)
        } else {
            println!("[SPOTIFY] Pending");
            Poll::Pending
        }
    }
}

pub async fn load_spotify_song(
    item: FullTrack,
    dlp_path: PathBuf,
    database: AM<Database>,
    music_path: PathBuf,
    thumbnail_path: PathBuf
) -> Result<Song, ()> {

    let artist = item.artists.into_iter().map(|artist| artist.name).collect::<Vec<String>>().join(" ");
    let search = format!("ytsearch1:{} {}", item.name, artist);

    let mut process = Command::new(dlp_path)
        .arg(search)
        .arg("--print")
        .arg("id")
        .stdout(Stdio::piped())
        .spawn()
        .map_err(|_| ())?;

    let stdout = process.stdout.take().ok_or(())?;
    let mut reader = BufReader::new(stdout).lines();

    let id = match reader.next_line().await {
        Ok(Some(line)) => line.trim().to_string(),
        _ => return Err(()),
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
