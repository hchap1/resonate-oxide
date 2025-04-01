use std::path::PathBuf;
use std::pin::Pin;
use std::task::Waker;
use std::thread::spawn;
use std::thread::JoinHandle;

use iced::futures::Stream;

use crate::frontend::message::Message;

use crate::backend::web::{collect_metadata, flatsearch};
use crate::backend::error::ResonateError;
use crate::backend::util::{sync, desync, AM};
use crate::backend::database::Database;
use crate::backend::music::Song;

pub async fn async_flatsearch(executable_dir: PathBuf, query: String) -> Message {
    match flatsearch(executable_dir, &query).await {
        Ok(results) => Message::LoadSearchResults(results),
        Err(_) => Message::DLPWarning
    }
}

pub fn populate(executable_dir: Option<PathBuf>, music_dir: PathBuf, thumbnail_dir: PathBuf, id: String, database: AM<Database>) -> Result<Song, ResonateError> {
    let song = collect_metadata(match executable_dir.as_ref() {
        Some(pathbuf) => Some(pathbuf.as_path()),
        None => None
    }, music_dir.as_path(), thumbnail_dir.as_path(), &id);

    match song {
        Ok(mut song) => {

            let database = desync(&database);
            let (success, id) = match database.emplace_song_and_record_id(&song, true) {
                Ok(data) => data,
                Err(_) => return Err(ResonateError::SQLError)
            };

            if success {
                song.id = id;
                Ok(song)
            } else {
                Err(ResonateError::GenericError(Box::new(String::from("Song already exists in table."))))
            }
        }
        Err(_) => return Err(ResonateError::SQLError)
    }
}

pub fn collect_metadata_and_notify_executor(
    executable_dir: Option<PathBuf>,
    music_dir: PathBuf,
    thumbnail_dir: PathBuf,
    id: String,
    database: AM<Database>,
    waker: AM<Option<Waker>> 
) -> Message {
    let song = match populate(executable_dir, music_dir, thumbnail_dir, id, database) {
        Ok(song) => song,
        Err(_) => {
            let waker_handle = desync(&waker);
            match waker_handle.as_ref() {
                Some(waker_handle) => waker_handle.wake_by_ref(),
                None => {}
            }
            return Message::None
        }
    };

    let waker_handle = desync(&waker);
    match waker_handle.as_ref() {
        Some(waker_handle) => waker_handle.wake_by_ref(),
        None => {}
    }

    Message::SearchResult(song)
}

pub struct AsyncMetadataCollectionPool {
    waker: AM<Option<Waker>>,               // The waker mutex, should be passed to the workers so they can notify the executor when they are ready to be collected
    thread_pool: Vec<JoinHandle<Message>>,  // Track the worker threads, poll whether they have ended or not
    queue: Vec<String>,                     // The list of songs that need to be collected

    executable_dir: Option<PathBuf>,        // Arguments required for parsing metadata which are shared for all songs
    music_dir: PathBuf,
    thumbnail_dir: PathBuf,
    database: AM<Database>
}

impl AsyncMetadataCollectionPool {
    pub fn new(ids: Vec<String>, executable_dir: Option<PathBuf>, music_dir: PathBuf, thumbnail_dir: PathBuf, database: AM<Database>) -> Self {
        Self {
            waker: sync(None),
            thread_pool: vec![],
            queue: ids,
            executable_dir,
            music_dir,
            thumbnail_dir,
            database
        }
    }
}

impl Stream for AsyncMetadataCollectionPool {
    type Item = Message;

    fn poll_next(mut self: Pin<&mut Self>, context: &mut std::task::Context<'_>) -> std::task::Poll<Option<<Self as Stream>::Item>> {
        'aquire_waker: {
            let mut waker = self.waker.lock().unwrap();
            if waker.is_some() { break 'aquire_waker; }
            *waker = Some(context.waker().clone());
        }

        if self.queue.len() == 0 && self.thread_pool.len() == 0 { return std::task::Poll::Ready(None) }

        let mut finished_workers: Vec<usize> = Vec::new();

        for (idx, worker) in self.thread_pool.iter().enumerate() {
            if worker.is_finished() {
                finished_workers.push(idx);
            }
        }

        let mut results: Option<Vec<Message>> = None;

        for (offset, idx) in finished_workers.iter().enumerate() {

            match self.thread_pool.remove(idx - offset).join() {
                Ok(message) => {
                    match results.as_mut() {
                        Some(results) => results.push(message),
                        None => results = Some(vec![message])
                    }
                }
                Err(_) => {}
            }

            if self.queue.len() == 0 { continue; }

            let executable = self.executable_dir.clone();
            let music = self.music_dir.clone();
            let thumbnails = self.thumbnail_dir.clone();
            let id = self.queue.pop().unwrap();
            let database = self.database.clone();
            let waker = self.waker.clone();

            self.thread_pool.push(spawn(
                move || collect_metadata_and_notify_executor(
                    executable,
                    music,
                    thumbnails,
                    id,
                    database,
                    waker
                )
            ));
        }

        if self.queue.len() > 0 && self.thread_pool.len() < 4 {
            let executable = self.executable_dir.clone();
            let music = self.music_dir.clone();
            let thumbnails = self.thumbnail_dir.clone();
            let id = self.queue.pop().unwrap();
            let database = self.database.clone();
            let waker = self.waker.clone();

            self.thread_pool.push(spawn(
                move || collect_metadata_and_notify_executor(
                    executable,
                    music,
                    thumbnails,
                    id,
                    database,
                    waker
                )
            ));
        }

        match results {
            Some(results) => std::task::Poll::Ready(Some(Message::MultiSearchResult(
                results.into_iter().filter_map(|message| match message {
                    Message::SearchResult(song) => Some(song),
                    _ => None
                }).collect()
            ))),
            None => std::task::Poll::Pending
        }
    }
}
