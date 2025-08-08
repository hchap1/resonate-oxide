use std::time::Duration;
use std::fmt::Formatter;
use std::path::PathBuf;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct Song {
    // Stored in database
    pub id: usize,
    pub yt_id: String,
    pub title: String,
    pub artist: String,
    pub album: Option<String>,
    pub duration: Duration,
    pub music_path: Option<PathBuf>
}

impl Song {
    pub fn new(
        id: usize, yt_id: String, title: String, artist: String, album: Option<String>, duration: Duration, music_dir: PathBuf
    ) -> Self {
        let mut s = Self { id, yt_id, title, artist, album, duration, music_path: None };
        s.load_music_path(music_dir);
        s
    }

    pub fn load_music_path(&mut self, music_dir: PathBuf) {
        let music_path = music_dir.join(format!("{}.mp3", self.yt_id));
        self.music_path = if music_path.exists() { Some(music_path) } else { None }
    }

    pub fn display_duration(&self) -> String {
        format!("{:02}:{:02}", 
            self.duration.as_secs() / 60,
            self.duration.as_secs() % 60
        )
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
