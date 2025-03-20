use std::path::Path;
use std::path::PathBuf;
use std::fs::read_dir;
use std::fs::create_dir_all;
use std::fs::create_dir;

use directories::ProjectDirs;
use youtube_dl::downloader::YoutubeDlFetcher;

use crate::backend::error::ResonateError;

pub struct DataDir {
    root: PathBuf,
    music: PathBuf,
    dependencies: PathBuf,
    thumbnails: PathBuf,
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
            Err(_) => return Err(error)
            
        };

        match create_dir(&dependencies) {
            Ok(_) => (),
            Err(_) => return Err(error)
        };

        match create_dir(&thumbnails) {
            Ok(_) => (),
            Err(_) => return Err(error)
        };

        Ok(Self { music, dependencies, thumbnails, root, dlp_path: None })
    }

    pub fn get_root_ref(&self) -> &Path { self.root.as_path() }
    pub fn get_music_ref(&self) -> &Path { self.music.as_path() }
    pub fn get_dependencies_ref(&self) -> &Path { self.dependencies.as_path() }
    pub fn get_thumbnails_ref(&self) -> &Path { self.thumbnails.as_path() }

    /// Attempt to install yt-dlp. If it is already installed, return the path
    pub async fn install_dlp(&self) -> Result<PathBuf, ResonateError> {

        // Check if some yt-dlp file already exists in the dependencies
        let existing_path_option = match read_dir(self.get_dependencies_ref()) {
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

        // If there is an existing executable, return the path to it
        if let Some(path) = existing_path_option { return Ok(path.path()); }

        // If not, attempt to download
        match YoutubeDlFetcher::default().download(self.get_dependencies_ref()).await {
            Ok(path) => Ok(path),
            Err(_) => Err(ResonateError::NetworkError(Box::new(String::from("Could not download YT-DLP"))))
        }
    }
}
