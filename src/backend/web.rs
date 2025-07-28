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

pub fn collect_metadata(
        executable_path: &Path,
        music_path: &Path,
        thumbnail_path: &Path,
        id: &String
    ) -> Result<Song, ResonateError> {

    let ytdl = YoutubeDl::new(id)
        .youtube_dl_path(executable_path)
        .extra_arg("--no-check-certificate")
        .extra_arg("--skip-download")
        .run();

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
pub struct AsyncMetadataCollectionPool {
    handle: Option<std::thread::JoinHandle<Result<Song, ResonateError>>>,
    dlp_path: PathBuf,
    music_path: PathBuf,
    thumbnail_path: PathBuf,
    database: DataLink,
    ids: Vec<String>
}

impl AsyncMetadataCollectionPool {
    pub fn new(
        database: DataLink,
        ids: Vec<String>,
        dlp_path: PathBuf,
        music_path: PathBuf,
        thumbnail_path: PathBuf
    ) -> Self {
        Self {
            handle: None,
            dlp_path,
            music_path,
            thumbnail_path,
            database,
            ids
        }
    }
}

fn populate(
    waker: Waker, id: String, database: DataLink, dlp_path: PathBuf,
    music_path: PathBuf, thumbnail_dir: PathBuf
) -> Result<Song, ResonateError> {

    if !DatabaseInterface::blocking_is_unique(database.clone(), id.clone()) {
        return Err(ResonateError::AlreadyExists);
    }

    let mut song = match collect_metadata(dlp_path.as_path(), music_path.as_path(), thumbnail_dir.as_path(), &id) {
        Ok(song) => song,
        error => return error
    };

    let id = match DatabaseInterface::blocking_insert_song(database, song.clone()) {
        Ok(id) => id,
        Err(_) => return Err(ResonateError::SQLError)
    };

    waker.wake();
    song.id = id;
    Ok(song)
}

impl Stream for AsyncMetadataCollectionPool {
    type Item = Result<Song, ()>;
    
    fn poll_next(
        mut self: Pin<&mut Self>, context: &mut std::task::Context<'_>
    ) -> std::task::Poll<Option<<Self as Stream>::Item>> {
        if self.handle.is_none() {
            match self.ids.pop() {

                // If no thread exists, and there are IDs left to populate, spawn a thread to do so
                Some(id) => {

                    let database = self.database.clone();
                    let dlp_path = self.dlp_path.clone();
                    let music_path = self.music_path.clone();
                    let thumbnail_path = self.thumbnail_path.clone();

                    let waker = context.waker().to_owned();

                    self.handle = Some(std::thread::spawn(
                        move || populate(
                            waker, id, database, dlp_path,
                            music_path, thumbnail_path
                        )
                    ))
                },

                // If no thread exists and there are no IDs left to populate, end the stream
                None => return std::task::Poll::Ready(None)
            }
        }

        let take_song = if let Some(handle) = self.handle.as_ref() {
            handle.is_finished()
        } else {
            false
        };

        if take_song {
            std::task::Poll::Ready(self.handle.take().unwrap().join().ok().map(|res| res.map_err(|_| ())))
        } else {
            std::task::Poll::Pending
        }
    }
}
