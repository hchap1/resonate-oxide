use std::collections::HashMap;

use std::path::PathBuf;
use std::path::Path;
use std::process::Command;
use std::collections::HashSet;
use std::fs::read_dir;

use async_channel::Sender;
use async_channel::Receiver;
use async_channel::unbounded;

use image::imageops::FilterType;

use crate::backend::music::Song;

pub type TnTx = Sender<ThumbnailMessage>;
pub type TnRx = Receiver<ThumbnailMessage>;
pub type TnPaths = (PathBuf, PathBuf);

#[derive(Clone, Debug)]
pub enum ThumbnailError {
    FailedToDownload,
    FailedToSpawnDLP,
    FailedToSave,
    FailedToRecv
}

impl From<async_channel::RecvError> for ThumbnailError {
    fn from(_: async_channel::RecvError) -> Self {
        Self::FailedToRecv
    }
}

#[derive(Clone, Debug)]
pub enum ThumbnailMessage {
    RequestDownload(Song, Sender<Result<Thumbnail, ThumbnailError>>),
    RequestPath(Song, Sender<Option<Thumbnail>>),
    InternalReturnDownload(Song, Result<Thumbnail, ThumbnailError>, Sender<Result<Thumbnail, ThumbnailError>>),
}

#[derive(Clone, Debug)]
pub struct Thumbnail {
    thumbnail: PathBuf,
    fullsize: PathBuf,
    blurred: PathBuf
}

impl Thumbnail {
    pub fn small(&self) -> &Path { &self.thumbnail }
}

pub struct ThumbnailManager {
    _handle: std::thread::JoinHandle<()>,
    task_sender: TnTx,
    default_thumbnail: PathBuf
}

impl ThumbnailManager {
    pub fn new(dlp_path: &Path, thumbnail_dir: &Path) -> Self {
        let (tx, rx) = unbounded();
        let passoff_tx = tx.clone();
        let paths = (dlp_path.to_path_buf(), thumbnail_dir.to_path_buf());
        Self {
            _handle: std::thread::spawn(
                 move || Self::run_thread(passoff_tx, rx, paths)
             ),
             task_sender: tx,
             default_thumbnail: thumbnail_dir.join("default_thumbnail.png")
        }
    }

    pub fn get_default(&self) -> Thumbnail {
        Thumbnail { 
            thumbnail: self.default_thumbnail.to_path_buf(),
            fullsize: self.default_thumbnail.to_path_buf(),
            blurred: self.default_thumbnail.to_path_buf(),
        }
    }

    pub fn download_thumbnail(&self, song: Song) -> impl std::future::Future<Output = Result<Thumbnail, ThumbnailError>> {
        let (tx, rx) = unbounded();
        let _ = self.task_sender.send_blocking(ThumbnailMessage::RequestDownload(song, tx));

        async move {
            rx.recv().await.map_err(ThumbnailError::from)?
        }
    }

    pub fn get_thumbnail_path_blocking(&self, song: Song) -> Thumbnail {
        let (tx, rx) = unbounded();
        let _ = self.task_sender.send_blocking(ThumbnailMessage::RequestPath(song, tx));
        match rx.recv_blocking().ok() {
            Some(Some(t)) => t,
            _ => self.get_default()
        }
    }

    pub fn download_thread(task_sender: TnTx, task_receiver: TnRx, paths: TnPaths) {
        while let Ok(task) = task_receiver.recv_blocking() {
            match task {
                ThumbnailMessage::RequestDownload(song, callback) => {
                    let _ = task_sender.send_blocking(
                        ThumbnailMessage::InternalReturnDownload(
                            song.clone(),
                            Self::download_thumbnails(&song, paths.0.as_path(), paths.1.as_path()),
                            callback
                        )
                    );
                },
                _ => println!("[THUMBNAIL DOWNLOADER] Received unsanctioned task")
            }
        }
    }

    pub fn run_thread(task_sender: TnTx, task_receiver: TnRx, paths: TnPaths) {
        let mut downloaded: HashMap<String, Thumbnail> = HashMap::new();
        let mut downloading: HashSet<String> = HashSet::new();
        let mut saved_paths: Vec<PathBuf> = Vec::new();

        if let Ok(contents) = read_dir(&paths.1) {
            for item in contents.flatten() {
                let path = item.path();
                if path.is_dir() {
                    saved_paths.push(path);
                }
            }
        }

        for path in saved_paths {
            let thumbnail = path.join("thumbnail.png");
            let fullsize = path.join("fullsize.png");
            let blurred = path.join("blurred.png");

            if !(thumbnail.exists() && fullsize.exists() && blurred.exists()) {
                continue;
            }

            let identifier = if let Some(identifier) = path.file_name() { identifier } else { continue };

            let thumbnail_struct = Thumbnail {
                thumbnail, fullsize, blurred
            };
            
            downloaded.insert(identifier.to_string_lossy().to_string(), thumbnail_struct);
        }

        let (download_sender, download_receiver) = unbounded();
        let _handle = std::thread::spawn(move || Self::download_thread(task_sender, download_receiver, paths));

        while let Ok(task) = task_receiver.recv_blocking() {
            match task {
                ThumbnailMessage::RequestDownload(song, callback) => {
                    // First check if already downloaded
                    let identifier = song.get_thumbnail_identifier();
                    if downloaded.contains_key(&identifier) || downloading.contains(&identifier) {
                        continue;
                    }

                    // If not already exists, then send it off to the downloader thread
                    let _ = downloading.insert(identifier);
                    let _ = download_sender.send_blocking(
                        ThumbnailMessage::RequestDownload(song, callback)
                    );
                },

                // The download thread has finished downloading something
                ThumbnailMessage::InternalReturnDownload(song, result, callback) => {
                    // Remove the song from the downloading list
                    let identifier = song.get_thumbnail_identifier();
                    if !downloading.remove(&identifier) {
                        println!("[THUMBNAIL] Received confirmation of untracked download");
                    }

                    // If the download was successful, save it
                    match result.as_ref() {
                        Ok(thumbnail) => { let _ = downloaded.insert(identifier, thumbnail.clone()); },
                        Err(e) => println!("[THUMBNAIL] Failed to download thumbnail: {e:?}")
                    };

                    let _ = callback.send_blocking(result);
                },

                ThumbnailMessage::RequestPath(song, callback) => {
                    // Grab the identifier and check if it exists
                    let identifier = song.get_thumbnail_identifier();
                    let _ = callback.send_blocking(downloaded.get(&identifier).cloned());
                }
            }
        }
    }


    fn download_thumbnails(song: &Song, dlp_path: &Path, thumbnail_dir: &Path) -> Result<Thumbnail, ThumbnailError> {
        // Get the thing this thumbnail will be saved as
        let identifier = song.get_thumbnail_identifier();
        let webp_path = thumbnail_dir.join(identifier.as_str());

        let mut ytdlp = Command::new(dlp_path);
        ytdlp.arg("--write-thumbnail")
            .arg("--skip-download")
            .arg("--no-check-certificate")
            .arg(format!("https://music.youtube.com/watch?v={}", song.yt_id))
            .arg("-o")
            .arg(webp_path);

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            ytdlp = ytdlp.creation_flags(0x08000000);
        }

        if let Ok(mut child) = ytdlp.spawn() {
            let _ = child.wait();
        } else {
            println!("[THUMBNAIL DOWNLOADER] Failed to spawn yt-dlp");
            return Err(ThumbnailError::FailedToSpawnDLP);
        }

        let raw = match image::open(thumbnail_dir.join(format!("{identifier}.webp"))) {
            Ok(image) => image,
            Err(_) => return Err(ThumbnailError::FailedToDownload)
        };

        let full_path = thumbnail_dir.join(&identifier);
        if !full_path.exists() {
            let res = std::fs::create_dir_all(&full_path);
            if res.is_err() { return Err(ThumbnailError::FailedToSave); }
        }

        let original_width = raw.width();
        let original_height = raw.height();

        // small size
        let new_height = 64;
        let new_width = (original_width as f64 * (new_height as f64 / original_height as f64)) as u32;
        let scaled = raw.resize(new_width, new_height, FilterType::Gaussian);

        let height = scaled.height();
        let padding = (scaled.width() - height) / 2;
        let cropped = scaled.crop_imm(padding, 0, height, height);
        let result = full_path.join("thumbnail.png");
        let _ = cropped.save(&result);

        // full size
        let size = original_width.min(original_height);
        let x_offset = (original_width - size) / 2;
        let y_offset = (original_height - size) / 2;
        let square_cropped = raw.crop_imm(x_offset, y_offset, size, size);
        let fullsize_path = full_path.join("fullsize.png");
        let _ = square_cropped.save(&fullsize_path);

        // blurred
        let blurred = square_cropped.blur(25.0);
        let blurred_path = full_path.join("blurred.png");
        let _ = blurred.save(&blurred_path);

        // delete webp
        let _ = std::fs::remove_file(thumbnail_dir.join(format!("{identifier}.webp")));

        match result.exists() && fullsize_path.exists() && blurred_path.exists() {
            true => Ok(Thumbnail {
                thumbnail: result, fullsize: fullsize_path, blurred: blurred_path
            }),
            false => Err(ThumbnailError::FailedToSave)
        }
    }
}
