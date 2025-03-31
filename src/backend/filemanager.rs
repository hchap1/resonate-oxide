use std::io::ErrorKind;
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

pub struct RefPackage<'a> {
    root: &'a Path,
    music: &'a Path,
    dependencies: &'a Path,
    thumbnails: &'a Path,
    dlp_path: Option<&'a Path>
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

        Ok(Self { music, dependencies, thumbnails, root, dlp_path })
    }

    pub fn get_root_ref(&self) -> &Path { self.root.as_path() }
    pub fn get_music_ref(&self) -> &Path { self.music.as_path() }
    pub fn get_dependencies_ref(&self) -> &Path { self.dependencies.as_path() }
    pub fn get_thumbnails_ref(&self) -> &Path { self.thumbnails.as_path() }
    pub fn get_dlp_ref(&self) -> Option<&Path> { self.dlp_path.as_ref().map(|v| &**v) }

    /// Attempt to install yt-dlp. If it is already installed, return the path
    pub async fn install_dlp(&mut self) -> Result<bool, ResonateError> {

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
        if let Some(path) = existing_path_option {
            self.dlp_path = Some(path.path());
        }

        // If not, attempt to download
        match YoutubeDlFetcher::default().download(self.get_dependencies_ref()).await {
            Ok(path) => { self.dlp_path = Some(path); Ok(true) }
            Err(_) => Err(ResonateError::NetworkError(Box::new(String::from("Could not download YT-DLP"))))
        }
    }

    pub fn ref_package(&self) -> RefPackage {
        RefPackage {
            root: &self.get_root_ref(),
            music: &self.get_music_ref(),
            dependencies: &self.get_dependencies_ref(),
            thumbnails: &self.get_thumbnails_ref(),
            dlp_path: self.get_dlp_ref()
        }
    }
}
