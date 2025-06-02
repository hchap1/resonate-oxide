use std::io::ErrorKind;
use std::path::Path;
use std::path::PathBuf;
use std::fs::read_dir;
use std::fs::create_dir_all;
use std::fs::create_dir;

use image::Luma;
use image::ImageBuffer;
use directories::ProjectDirs;
use youtube_dl::downloader::YoutubeDlFetcher;

use crate::backend::error::ResonateError;

#[derive(Clone)]
pub struct DataDir {
    root: PathBuf,
    music: PathBuf,
    dependencies: PathBuf,
    thumbnails: PathBuf,
    default_thumbnail: PathBuf,
    dlp_path: Option<PathBuf>
}

impl DataDir {
    pub fn create_or_load() -> Result<Self, ResonateError> {
        let root = match ProjectDirs::from("com", "hchap1", "ResonateData") {
            Some(dir_builder) => dir_builder.data_dir().to_path_buf(),
            None => return Err(ResonateError::UnrecognisedHomeDir)
        };

        let error = ResonateError::DirectoryNotFound(Box::new(format!("{root:?} could not be created")));
        let music = root.join("music");
        let dependencies = root.join("dependencies");
        let thumbnails = root.join("thumbnails");

        let _ = create_dir_all(&root);
        if !root.exists() { return Err(error); }

        match create_dir(&music) {
            Ok(_) => (),
            Err(e) if e.kind() == ErrorKind::AlreadyExists => (),
            Err(_) => return Err(error)
        };

        match create_dir(&dependencies) {
            Ok(_) => (),
            Err(e) if e.kind() == ErrorKind::AlreadyExists => (),
            Err(_) => return Err(error)
        };

        match create_dir(&thumbnails) {
            Ok(_) => (),
            Err(e) if e.kind() == ErrorKind::AlreadyExists => (),
            Err(_) => return Err(error)
        };

        let default_thumbnail = if thumbnails.exists() {
            let default_thumbnail = thumbnails.join("default_thumbnail.png");
            if !default_thumbnail.exists() {
                let img: ImageBuffer<Luma<u8>, Vec<u8>> = ImageBuffer::from_fn(64, 64, |_, _| Luma([100]));
                let _ = img.save(&default_thumbnail);
            }
            default_thumbnail
        } else {
            panic!("COULDN'T CREATE DEFAULT THUMBNAIL");
        };

        let mut matching_entry = match read_dir(&dependencies) {
            Ok(entries) => entries,
            Err(_) => return Err(ResonateError::DirectoryNotFound(Box::new(String::from("Could not read from the dependencies directory."))))
        }.find(|entry| match entry {
                Err(_) => false,
                Ok(entry) => {
                    if entry.path().to_string_lossy().to_string().contains("yt-dlp") {
                        true
                    } else {
                        false
                    }
                }
            });

        let dlp_path = match matching_entry.take() {
            Some(entry) => Some(entry.unwrap().path().to_path_buf()),
            None => None
        };

        Ok(Self { music, dependencies, thumbnails, root, dlp_path, default_thumbnail })
    }

    pub fn take_dlp_path(&mut self, dlp_path: PathBuf) {
        self.dlp_path = Some(dlp_path);
    }

    pub fn get_root_ref(&self) -> &Path { self.root.as_path() }
    pub fn get_music_ref(&self) -> &Path { self.music.as_path() }
    pub fn get_dependencies_ref(&self) -> &Path { self.dependencies.as_path() }
    pub fn get_thumbnails_ref(&self) -> &Path { self.thumbnails.as_path() }
    pub fn get_dlp_ref(&self) -> Option<&Path> {
        match &self.dlp_path {
            Some(dlp_path) => Some(dlp_path.as_path()),
            None => None
        }
    }
    pub fn get_default_thumbnail(&self) -> &Path { self.default_thumbnail.as_path() }

}

/// Attempt to install yt-dlp. If it is already installed, return the path
pub async fn install_dlp(target: PathBuf) -> Result<PathBuf, ResonateError> {
    println!("[DIRECTORIES] Attempting to download yt-dlp to {target:?}");
    // Check if some yt-dlp file already exists in the dependencies
    let existing_path_option = match read_dir(&target) {
        Ok(contents) => contents.into_iter().filter_map(|item| match item {
            Ok(item) => Some(item),
            Err(_) => None
        }).find(|entry| {
            match entry.path().is_file() {
                true => {
                    entry.path().to_string_lossy().to_string().contains("yt-dlp")
                }
                false => false
            }
        }),
        Err(e) => return Err(ResonateError::DirectoryNotFound(Box::new(e)))
    };

    println!("[DIRECTORIES] Existing installation: {existing_path_option:?}");

    // If there is an existing executable, return the path to it
    if let Some(path) = existing_path_option {
        return Ok(path.path());
    }

    // If not, attempt to download
    match YoutubeDlFetcher::default().download(target).await {
        Ok(path) => Ok(path),
        Err(_) => Err(ResonateError::NetworkError(Box::new(String::from("Could not download YT-DLP"))))
    }
}
