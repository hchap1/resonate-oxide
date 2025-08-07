use std::time::Duration;
use std::fmt::Formatter;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Song {
    // Stored in database
    pub id: usize,
    pub yt_id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration: Duration,
    pub music_path: Option<std::path::PathBuf>
}

impl Song {
    pub fn new( id: usize, yt_id: String, title: String, artist: String, album: Option<String>, duration: Duration) -> Self {
        Self { id, yt_id, title, artist, album, duration, music_path: None }
    }

    pub fn display_duration(&self) -> String {
        format!("{:02}:{:02}", 
            self.duration.as_secs() / 60,
            self.duration.as_secs() % 60
        )
    }

    /// Convenience method
    pub fn get_music_identifier(&self) -> String {
        self.yt_id.clone()
    }

    /// This is what is always used to produce the album
    pub fn get_thumbnail_identifier(&self) -> String {
        match self.album.as_ref() {
            Some(album) => album.replace(" ", "_"),
            None => self.yt_id.replace(" ", "_")
        }
    }
}

impl std::fmt::Display for Song {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        let album_string = match self.album.as_ref() {
            Some(album) => format!(" in {}", album),
            None => String::new()
        };
        write!(f, "{} by {}{album_string}. {:02}{:02}. SQL: {}. YT: {}.",
            self.title,
            self.artist,
            self.duration.as_secs() / 60,
            self.duration.as_secs() % 60,
            self.id,
            self.yt_id,
        )
    }
}

#[derive(Clone, Debug)]
pub struct Playlist {
    pub id: usize,
    pub name: String,
}
