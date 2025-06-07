use rustfm_scrobble::Scrobble;
use rustfm_scrobble::Scrobbler;
use rustfm_scrobble::ScrobblerError;

use crossbeam_channel::Sender;
use crossbeam_channel::Receiver;
use crossbeam_channel::unbounded;

use std::fmt::Debug;
use std::fmt::Formatter;
use std::fmt;

use tokio::task::spawn_blocking;

use super::music::Song;

#[derive(Debug, Clone)]
pub struct FMAuth {
    pub key: Option<String>,
    pub secret: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>
}

impl FMAuth {
    pub fn new(
        key: Option<String>, secret: Option<String>, username: Option<String>, password: Option<String>
    ) -> FMAuth {
        FMAuth { key, secret, username, password }
    }
}

#[derive(Debug)]
pub enum FMError {
    MissingFields,
    AuthenticationError(ScrobblerError),
    TokioError
}

impl Clone for FMError {
    fn clone(&self) -> FMError {
        match self {
            FMError::MissingFields => FMError::MissingFields,
            FMError::TokioError => FMError::TokioError,
            FMError::AuthenticationError(_) => FMError::AuthenticationError(ScrobblerError::new(String::new()))
        }
    }
}

#[derive(Clone, Debug)]
pub enum FMMessage {
    SetNowPlaying(Song),
    SetScrobbling(Song)
}

#[derive(Clone)]
pub struct LastFM {
    pub scrobbler: &'static Scrobbler,
    sender: Sender<FMMessage>,
    receiver: Receiver<FMMessage>
}

impl Debug for LastFM {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "LastFM Object")
    }
}

impl LastFM {
    pub async fn new(auth: FMAuth) -> Result
        <(LastFM, Receiver<FMMessage>, Sender<FMMessage>),
        FMError>
    {
        let scrobbler = match spawn_blocking(move || authenticate(auth)).await {
            Ok(scrobbler) => match scrobbler {
                Ok(scrobbler) => scrobbler,
                Err(e) => return Err(e)
            },
            Err(_) => return Err(FMError::TokioError)
        };

        let (sender, thread_receiver) = unbounded();
        let (thread_sender, receiver) = unbounded();

        Ok((LastFM {
            // handle: spawn(move || scrobble_thread(scrobbler, thread_receiver, thread_sender)),
            sender, receiver, scrobbler: Box::<Scrobbler>::leak(Box::<Scrobbler>::new(scrobbler))
        }, thread_receiver, thread_sender))
    }

    pub async fn send_task(&self, message: FMMessage) {
        let _ = self.sender.send(message);
    }
}

fn authenticate(mut auth: FMAuth) -> Result<Scrobbler, FMError> {
    let key = if let Some(val) = auth.key.take() { val } else { return Err(FMError::MissingFields) };
    let secret = if let Some(val) = auth.key.take() { val } else { return Err(FMError::MissingFields) };
    let username = if let Some(val) = auth.username.take() { val } else { return Err(FMError::MissingFields) };
    let password = if let Some(val) = auth.password.take() { val } else { return Err(FMError::MissingFields) };

    let mut scrobbler = Scrobbler::new(key.as_str(), secret.as_str());
    match scrobbler.authenticate_with_password(username.as_str(), password.as_str()) {
        Ok(_session_response) => {},
        Err(e) => return Err(FMError::AuthenticationError(e))
    }

    Ok(scrobbler)
}

fn song_to_scrobble(song: &Song) -> Scrobble {
    Scrobble::new(song.artist.as_str(), song.title.as_str(), song.album.as_ref().map_or("None", |v| v.as_str()))
}

pub fn scrobble_thread(scrobbler: &'static Scrobbler, receiver: Receiver<FMMessage>, sender: Sender<FMMessage>) {
    loop {
        match receiver.recv() {
            Ok(fm_message) => match fm_message {
                FMMessage::SetNowPlaying(song) => {
                    let _ = scrobbler.now_playing(&song_to_scrobble(&song));
                },
                FMMessage::SetScrobbling(song) => {
                    let _ = scrobbler.scrobble(&song_to_scrobble(&song));
                }
            },
            Err(_) => return
        }
    }
}
