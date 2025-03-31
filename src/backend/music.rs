use std::path::PathBuf;
use std::time::Duration;
use std::fmt::Formatter;

#[derive(Debug, Clone)]
pub struct Song {
    // Stored in database
    pub id: usize,
    pub yt_id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration: Duration,

    // Derived upon retrieval from database
    pub music_path: Option<PathBuf>,
    pub thumbnail_path: Option<PathBuf>
}

impl Song {
    pub fn new(
        id: usize,
        yt_id: String,
        title: String,
        artist: String,
        album: Option<String>,
        duration: Duration,
        music_path: Option<PathBuf>,
        thumbnail_path: Option<PathBuf>
    ) -> Self {
        Self {
            id,
            yt_id,
            title,
            artist,
            album,
            duration,
            music_path,
            thumbnail_path
        }
    }
}

impl Song {
    pub fn display_duration(&self) -> String {
        format!("{:02}{:02}", 
            self.duration.as_secs() / 60,
            self.duration.as_secs() % 60
        )
    }
}

impl std::fmt::Display for Song {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        let album_string = match self.album.as_ref() {
            Some(album) => format!(" in {}", album),
            None => String::new()
        };

        let music_path = match self.music_path.as_ref() {
            Some(music_path) => format!("Downloaded to {}.", music_path.to_string_lossy().to_string()),
            None => String::from("Not downloaded.")
        };

        let thumbnail_path = match self.thumbnail_path.as_ref() {
            Some(thumbnail_path) => format!("Thumbnail downloaded to {}.", thumbnail_path.to_string_lossy().to_string()),
            None => String::from("No thumbnail downloaded.")
        };

        write!(f, "{} by {}{album_string}. {:02}{:02}. SQL: {}. YT: {}. {} {}",
            self.title,
            self.artist,
            self.duration.as_secs() / 60,
            self.duration.as_secs() % 60,
            self.id,
            self.yt_id,
            music_path,
            thumbnail_path
        )
    }
}
