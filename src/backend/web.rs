use std::path::PathBuf;
use std::path::Path;
use std::time::Duration;
use std::task::Waker;
use std::pin::Pin;
use std::thread::JoinHandle;
use std::thread::spawn;

use iced::futures::Stream;
use image::imageops::FilterType;
use youtube_dl::YoutubeDl;
use tokio::process::Command;

use crate::backend::error::ResonateError;
use crate::backend::music::Song;
use crate::backend::database::Database;
use crate::backend::util::desync;
use crate::backend::util::sync;
use crate::backend::util::AM;

pub async fn flatsearch(
        executable_path: PathBuf,
        query: String
    ) -> Result<Vec<String>, ResonateError> {

    let url = format!("https://music.youtube.com/search?q={}", query.replace(" ", "+"));
    match YoutubeDl::new(url)
        .youtube_dl_path(executable_path)
        .extra_arg("--skip-download")
        .extra_arg("--flat-playlist")
        .run_async().await {
        Ok(result) => {
            match result.into_playlist() {
                Some(mut playlist) => match playlist.entries.take() {
                    Some(entries) => Ok(entries.into_iter().filter_map(|entry| {
                        match entry.title {
                            Some(_) => Some(entry.id.clone()),
                            None => None
                        }
                    }).collect::<Vec<String>>()),
                    None => Ok(Vec::<String>::new())
                }
                None => Ok(Vec::<String>::new())
            }
        }
        Err(e) => {
            println!("{e:?}");
            return Err(ResonateError::NetworkError)
        }
    }
}

pub fn collect_metadata(
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
                        Ok(Song::load(0, entry.id, title, artist, entry.album.take(), Duration::from_secs(duration), music_path, thumbnail_path))
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
        .arg(url)
        .spawn().unwrap();

    let status = ytdlp.wait().await;
    println!("[YT-DLP] Status: {status:?}");

    match output.exists() {
        true => Ok(song),
        false => Err(song)
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
                Err(ResonateError::GenericError)
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
) -> Result<Song, ResonateError> {
    let song = match populate(executable_dir, music_dir, thumbnail_dir, id, database) {
        Ok(song) => song,
        Err(_) => {
            let waker_handle = desync(&waker);
            match waker_handle.as_ref() {
                Some(waker_handle) => waker_handle.wake_by_ref(),
                None => {}
            }
            return Err(ResonateError::NetworkError)
        }
    };

    let waker_handle = desync(&waker);
    match waker_handle.as_ref() {
        Some(waker_handle) => waker_handle.wake_by_ref(),
        None => {}
    }

    Ok(song)
}

pub struct AsyncMetadataCollectionPool {
    waker: AM<Option<Waker>>,               // The waker mutex, should be passed to the workers so they can notify the executor when they are ready to be collected
    thread_pool: Vec<JoinHandle<Result<Song, ResonateError>>>,  // Track the worker threads, poll whether they have ended or not
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
    type Item = Vec<Song>;

    fn poll_next(
        mut self: Pin<&mut Self>, context: &mut std::task::Context<'_>
    ) -> std::task::Poll<Option<<Self as Stream>::Item>> {
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

        let mut results: Option<Vec<Song>> = None;

        for (offset, idx) in finished_workers.iter().enumerate() {

            match self.thread_pool.remove(idx - offset).join() {
                Ok(res) => {
                    if let Some(results) = results.as_mut() {
                        if let Ok(song) = res {
                            results.push(song);
                        }
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
            Some(results) => std::task::Poll::Ready(Some(results)),
            None => std::task::Poll::Pending
        }
    }
}
