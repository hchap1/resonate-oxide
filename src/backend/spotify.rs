use std::path::PathBuf;
use std::time::Duration;

use std::process::Stdio;

use std::pin::Pin;
use std::task::Poll;
use std::task::Context;
use std::task::Waker;

use pin_project::pin_project;

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

use async_channel::Receiver;
use async_channel::Sender;
use async_channel::unbounded;

use super::error::ResonateError;
use super::music::Song;
use super::database_manager::DataLink;
use super::database_interface::DatabaseInterface;

pub async fn try_auth(credentials: ClientCredsSpotify) -> Result<ClientCredsSpotify, ()> {
    match credentials.request_token().await {
        Ok(_) => Ok(credentials),
        Err(_) => Err(())
    }
}

#[pin_project]
pub struct SpotifySongStream {
    handle: JoinHandle<Result<(), ()>>,
    sender: Sender<InterThreadMessage>,
    receiver: Receiver<InterThreadMessage>,
    waker_received: bool,

    result_count: usize,
    playlist_name: String,
    has_streamed_result_count: bool
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
            waker_received: false,
            result_count: 0,
            playlist_name: String::new(),
            has_streamed_result_count: false
        }
    }
}

enum InterThreadMessage {
    Done,
    Result(Box<PlaylistItem>),
    Waker(Waker),
    WakerReceived,
    PlaylistName(String, usize),
    InvalidID
}

async fn consume_stream(
    playlist_link: String,
    credentials: ClientCredsSpotify,
    receiver: Receiver<InterThreadMessage>,
    sender: Sender<InterThreadMessage>
) -> Result<(), ()> {
    let duration = Duration::from_millis(100);
    let waker = 'outer: loop {
        let _ = tokio::time::sleep(duration).await;

        while let Ok(message) = receiver.try_recv() {
            match message {
                InterThreadMessage::Done => return Err(()), // shouldnt happen
                InterThreadMessage::Waker(waker) => {
                    break 'outer waker
                }
                _ => continue
            }
        }
    };

    let playlist_id = match PlaylistId::from_id_or_uri(playlist_link.as_str()) {
        Ok(playlist_id) => playlist_id,
        Err(_) => {
            let _ = sender.send(InterThreadMessage::InvalidID).await;
            waker.wake_by_ref();
            return Err(())
        }
    };

    let mut stream = credentials.playlist_items(
        playlist_id.clone(),
        None,
        Some(rspotify::model::Market::Country(rspotify::model::Country::UnitedStates)),
    );

    match sender.send_blocking(InterThreadMessage::WakerReceived) {
        Ok(_) => {},
        Err(_) => {
            waker.wake_by_ref();
            return Err(())
        }
    }

    let playlist = credentials.playlist(
        playlist_id,
        None,
        Some(rspotify::model::Market::Country(rspotify::model::Country::UnitedStates)),
    ).await;

    if let Ok(playlist) = playlist {
        let _ = sender.send_blocking(InterThreadMessage::PlaylistName(playlist.name, playlist.tracks.items.len()));
    }

    loop {
        match stream.next().await {
            Some(playlist_item) => match playlist_item {
                Ok(playlist_item) => match sender.send_blocking(
                        InterThreadMessage::Result(Box::new(playlist_item))
                    ) {
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
                    let _ = sender.send_blocking(InterThreadMessage::Done);
                    println!("[SPOTIFY] Async thread exiting because Paginator stream failed to produce item");
                    waker.wake_by_ref();
                    return Err(())
                }
            }   
            None => {
                let _ = sender.send_blocking(InterThreadMessage::Done);
                println!("[SPOTIFY] Async thread exiting because Paginator stream failed");
                waker.wake_by_ref();
                return Err(())
            }
        }

        waker.wake_by_ref();
    }
}

pub enum SpotifyEmmision {
    Item(Box<PlaylistItem>),
    Name(String, usize),
    IDFailure
}

impl Stream for SpotifySongStream {
    type Item = SpotifyEmmision;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut Context<'_>) -> Poll<Option<SpotifyEmmision>> {
        if !self.waker_received {
            let _ = self.sender.send_blocking(InterThreadMessage::Waker(context.waker().to_owned()));
        }

        loop {
            match self.receiver.try_recv() {
                Ok(message) => match message {
                    InterThreadMessage::Done => {
                        if self.has_streamed_result_count {
                            return Poll::Ready(None) // done
                        } else {
                            self.has_streamed_result_count = true;
                            return Poll::Ready(Some(SpotifyEmmision::Name(
                                self.playlist_name.clone(), self.result_count
                            )));
                        }
                    }
                    InterThreadMessage::Result(res) => {
                        self.result_count += 1;
                        return Poll::Ready(Some(SpotifyEmmision::Item(res)))
                    },
                    InterThreadMessage::WakerReceived => {
                        self.waker_received = true;
                    }
                    InterThreadMessage::PlaylistName(name, size) => {
                        self.playlist_name = name.clone();
                        return Poll::Ready(Some(SpotifyEmmision::Name(name, size)))
                    }
                    InterThreadMessage::InvalidID => {
                        return Poll::Ready(Some(SpotifyEmmision::IDFailure))
                    }
                    _ => {}
                },
                Err(async_channel::TryRecvError::Closed) => return Poll::Ready(None),
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
    database: DataLink,
    music_path: PathBuf
) -> Result<Song, ResonateError> {

    let artist = item.artists.into_iter().map(|artist| artist.name).collect::<Vec<String>>().join(" ");
    let search = format!("ytsearch1:{} {}", item.name, artist);

    let mut process = Command::new(dlp_path);
    process.arg(search)
        .arg("--print")
        .arg("id")
        .stdout(Stdio::piped());

    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        ytdlp = ytdlp.creation_flags(0x08000000);
    }
        
    let mut process = process.spawn().map_err(|_| ResonateError::ExecNotFound)?;
    let stdout = process.stdout.take().ok_or(ResonateError::STDOUTError)?;
    let mut reader = BufReader::new(stdout).lines();

    let id = match reader.next_line().await {
        Ok(Some(line)) => line.trim().to_string(),
        _ => return Err(ResonateError::STDOUTError),
    };

    if id.len() != 11 {
        return Err(ResonateError::STDOUTError);
    }

    let mut base_song = Song::new(
        0, id,
        item.name,
        artist,
        Some(item.album.name),
        item.duration.to_std().unwrap_or(Duration::from_secs(0)),
        music_path
    );

    let id = match DatabaseInterface::insert_song(database, base_song.clone()).await {
        Some(data) => data,
        None => return Err(ResonateError::SQLError)
    };
    

    base_song.id = id;
    Ok(base_song)
}

pub fn extract_spotify_playlist_id(url: String) -> Option<String> {
    url.split('/')
        .nth(4)
        .map(|s| s.split('?').next().unwrap_or(s))
        .filter(|id| id.len() == 22)
        .map(|id| id.to_string())
}
