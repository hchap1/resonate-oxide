use std::path::PathBuf;
use std::path::Path;
use std::time::Duration;
use std::task::Waker;
use std::pin::Pin;

use crossbeam_channel::Sender;
use crossbeam_channel::Receiver;
use crossbeam_channel::unbounded;
use tokio::task::spawn;
use tokio::task::JoinHandle;

use iced::futures::Stream;
use image::imageops::FilterType;
use youtube_dl::YoutubeDl;
use tokio::process::Command;

use crate::backend::error::ResonateError;
use crate::backend::music::Song;
use crate::backend::database_manager::DataLink;

use super::database_interface::DatabaseInterface;

pub async fn flatsearch(
        executable_path: PathBuf,
        query: String
    ) -> Result<Vec<String>, ResonateError> {

    let url = format!("https://music.youtube.com/search?q={}", query.replace(" ", "+"));
    match YoutubeDl::new(url)
        .youtube_dl_path(executable_path)
        .extra_arg("--skip-download")
        .extra_arg("--flat-playlist")
        .extra_arg("--no-check-certificate")
        .run_async().await {
        Ok(result) => {
            match result.into_playlist() {
                Some(mut playlist) => match playlist.entries.take() {
                    Some(entries) => Ok(entries.into_iter().filter_map(|entry| {
                        match entry.title {
                            Some(_) => {
                                println!("SEARCH RESULT: {}", entry.title.unwrap());
                                Some(entry.id.clone())
                            }
                            None => None
                        }
                    }).collect::<Vec<String>>()),
                    None => {
                        println!("Flatsearch failed. Could not take entries.");
                        Ok(Vec::<String>::new())
                    }
                }
                None => {
                    println!("Flatsearch failed. Could not convert into playlist.");
                    Ok(Vec::<String>::new())
                }
            }
        }
        Err(e) => {
            println!("{e:?}");
            return Err(ResonateError::NetworkError)
        }
    }
}

pub async fn collect_metadata(
        executable_path: Option<&Path>,
        music_path: &Path,
        thumbnail_path: &Path,
        id: &String
    ) -> Result<Song, ResonateError> {

    let path = match executable_path {
        Some(path) => path,
        None => return Err(ResonateError::ExecNotFound)
    };

    let ytdl = YoutubeDl::new(id)
        .youtube_dl_path(path)
        .extra_arg("--no-check-certificate")
        .extra_arg("--skip-download")
        .run_async().await;

    match ytdl {
        Ok(result) => {
            match result.into_single_video() {
                Some(mut entry) => {
                    if let (Some(title), Some(artist), Some(duration)) = (entry.title, entry.artist, entry.duration) {
                        let duration = match duration.as_u64() {
                            Some(duration) => duration,
                            None => 0u64
                        };
                        Ok(
                            Song::load(
                                0,
                                entry.id,
                                title,
                                artist,
                                entry.album.take(),
                                Duration::from_secs(duration),
                                music_path.to_owned(),
                                thumbnail_path.to_owned()
                            )
                        )
                    } else {
                        Err(ResonateError::NetworkError)
                    }
                }
                None => Err(ResonateError::NetworkError)
            }
        }
        Err(_) => Err(ResonateError::NetworkError)
    }
}

pub async fn download_thumbnail(dlp_path: PathBuf, thumbnail_dir: PathBuf, id: String, album_name: String) -> Result<PathBuf, ()> {
    let album = album_name.replace(" ", "_");
    let path = thumbnail_dir.join(&album).to_string_lossy().to_string();

    let mut handle = Command::new(dlp_path)
        .arg("--write-thumbnail")
        .arg("--skip-download")
        .arg("--no-check-certificate")
        .arg(format!("https://music.youtube.com/watch?v={}", id))
        .arg("-o")
        .arg(path.clone())
        .spawn().unwrap();

    let _ = handle.wait().await;

    let raw = match image::open(thumbnail_dir.join(format!("{album}.webp"))) {
        Ok(image) => image,
        Err(_) => return Err(())
    };

    let original_width = raw.width();
    let original_height = raw.height();
    let new_height = 64;
    let new_width = (original_width as f64 * (new_height as f64 / original_height as f64)) as u32;
    let scaled = raw.resize(new_width, new_height, FilterType::Gaussian);

    let result: PathBuf = thumbnail_dir.join(format!("{album}.png"));

    let height = scaled.height();
    let padding = (scaled.width() - height)/2;
    let cropped = scaled.crop_imm(padding, 0, height, height);
    let _ = cropped.save(&result);
    let _ = std::fs::remove_file(thumbnail_dir.join(format!("{album}.webp")));

    match result.exists() {
        true => Ok(result),
        false => Err(())
    }
}

pub async fn download_song(dlp_path: Option<PathBuf>, music_path: PathBuf, song: Song) -> Result<Song, Song> {
    let dlp_path = match dlp_path {
        Some(dlp_path) => dlp_path,
        None => return Err(song)
    };

    let output = music_path.join(format!("{}.mp3", song.yt_id));
    let url = format!("https://music.youtube.com/watch?v={}", song.yt_id);

    let mut ytdlp = Command::new(dlp_path)
        .arg("-f")
        .arg("bestaudio")
        .arg("--extract-audio")
        .arg("--audio-format")
        .arg("mp3")
        .arg("-o")
        .arg(output.to_string_lossy().to_string())
        .arg("--no-check-certificate")
        .arg(url)
        .spawn().unwrap();

    let status = ytdlp.wait().await;
    println!("[YT-DLP] Status: {status:?}");

    match output.exists() {
        true => Ok(song),
        false => Err(song)
    }
}

pub async fn populate(
    executable_dir: Option<PathBuf>, music_dir: PathBuf, thumbnail_dir: PathBuf, id: String, database: DataLink
) -> Result<Song, ResonateError> {

    match DatabaseInterface::select_song_by_youtube_id(
        database.clone(), id.clone(), music_dir.clone(), thumbnail_dir.clone()
    ).await {
        Some(song) => {
            println!("SONG {:?} ALREADY EXISTS", song);
            return Err(ResonateError::AlreadyExists)
        }
        None => {}
    };

    let mut song = match collect_metadata(match executable_dir.as_ref() {
        Some(pathbuf) => Some(pathbuf.as_path()),
        None => None
    }, music_dir.as_path(), thumbnail_dir.as_path(), &id).await {
        Ok(song) => song,
        Err(_) => return Err(ResonateError::GenericError)
    };

    let id = match DatabaseInterface::insert_song(database.clone(), song.clone()).await {
        Some(id) => id,
        None => return Err(ResonateError::GenericError)
    };

    song.id = id;

    println!("INSERTED SONG: {song:?}");

    return Ok(song)
}

pub async fn collect_metadata_and_notify_executor(
    executable_dir: Option<PathBuf>,
    music_dir: PathBuf,
    thumbnail_dir: PathBuf,
    id: String,
    database: DataLink,
    waker: Waker,
    sender: Sender<Song>
) -> Result<(), ResonateError> {
    let song = match populate(executable_dir, music_dir, thumbnail_dir, id, database).await {
        Ok(song) => song,
        Err(_) => {
            waker.wake_by_ref();
            return Err(ResonateError::NetworkError)
        }
    };

    println!("WOKEN AND SENT: {song:?}");
    let _ = sender.send(song);
    waker.wake_by_ref();
    Ok(())
}

pub struct AsyncMetadataCollectionPool {
    thread_pool: Vec<JoinHandle<Result<(), ResonateError>>>,  // Track the worker threads, poll whether they have ended or not
    queue: Vec<String>,                     // The list of songs that need to be collected

    executable_dir: Option<PathBuf>,        // Arguments required for parsing metadata which are shared for all songs
    music_dir: PathBuf,
    thumbnail_dir: PathBuf,
    database: DataLink,
    sender: Sender<Song>,
    receiver: Receiver<Song>
}

impl AsyncMetadataCollectionPool {
    pub fn new(
        ids: Vec<String>, executable_dir: Option<PathBuf>,
        music_dir: PathBuf, thumbnail_dir: PathBuf, database: DataLink
    ) -> Self {

        let (sender, receiver) = unbounded();

        Self {
            thread_pool: vec![],
            queue: ids,
            executable_dir,
            music_dir,
            thumbnail_dir,
            database,
            sender,
            receiver
        }
    }
}

impl Stream for AsyncMetadataCollectionPool {
    type Item = Vec<Song>;

    fn poll_next(
        mut self: Pin<&mut Self>, context: &mut std::task::Context<'_>
    ) -> std::task::Poll<Option<<Self as Stream>::Item>> {
        let waker = context.waker().clone();
        if self.queue.len() == 0 && self.thread_pool.len() == 0 { return std::task::Poll::Ready(None) }

        let mut finished_workers: Vec<usize> = Vec::new();

        for (idx, worker) in self.thread_pool.iter().enumerate() {
            if worker.is_finished() {
                finished_workers.push(idx);
            }
        }

        let mut results = Vec::new();

        for (offset, idx) in finished_workers.iter().enumerate() {
            self.thread_pool.remove(idx - offset);
            if self.queue.len() == 0 { continue; }
            let executable = self.executable_dir.clone();
            let music = self.music_dir.clone();
            let thumbnails = self.thumbnail_dir.clone();
            let id = self.queue.pop().unwrap();
            let database = self.database.clone();
            let sender_clone = self.sender.clone();

            self.thread_pool.push(tokio::spawn(
                collect_metadata_and_notify_executor(
                    executable,
                    music,
                    thumbnails,
                    id,
                    database,
                    waker.clone(),
                    sender_clone
                )
            ));
        }

        if self.queue.len() > 0 && self.thread_pool.len() < 4 {
            let executable = self.executable_dir.clone();
            let music = self.music_dir.clone();
            let thumbnails = self.thumbnail_dir.clone();
            let id = self.queue.pop().unwrap();
            let database = self.database.clone();
            let waker = waker.clone();
            let sender_clone = self.sender.clone();

            self.thread_pool.push(spawn(
                collect_metadata_and_notify_executor(
                    executable,
                    music,
                    thumbnails,
                    id,
                    database,
                    waker,
                    sender_clone
                )
            ));
        }

        while let Ok(song) = self.receiver.try_recv() {
            results.push(song);
        }

        match results.len() {
            0 => std::task::Poll::Ready(Some(results)),
            _ => std::task::Poll::Pending
        }
    }
}
