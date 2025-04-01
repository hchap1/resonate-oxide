use std::path::PathBuf;
use std::path::Path;
use std::time::Duration;

use youtube_dl::YoutubeDl;

use crate::backend::error::ResonateError;
use crate::backend::music::Song;

pub async fn flatsearch(
        executable_path: PathBuf,
        query: &str
    ) -> Result<Vec<String>, ResonateError> {

    let url = format!("https://music.youtube.com/search?q={}", query.replace(" ", "+"));
    match YoutubeDl::new(url)
        .youtube_dl_path(executable_path)
        .extra_arg("--skip-download")
        .extra_arg("--flat-playlist")
        .run() {
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
            return Err(ResonateError::NetworkError(Box::new(String::from("Failed to search with yt-dlp"))))
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
        None => return Err(ResonateError::ExecNotFound(Box::new(String::from("yt-dlp not installed."))))
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
                        let music = music_path.join(format!("{}.mp3", entry.id));
                        let music = if music.exists() { Some(music) } else { None };
                        let thumbnail = match entry.album.as_ref() {
                            Some(album) => {
                                let path = thumbnail_path.join(format!("{}.png", album.replace(' ', "_")));
                                if path.exists() { Some(path) } else { None }
                            }
                            None => None
                        };
                        let duration = match duration.as_u64() {
                            Some(duration) => duration,
                            None => 0u64
                        };
                        Ok(Song::new(0, entry.id, title, artist, entry.album.take(), Duration::from_secs(duration), music, thumbnail))
                    } else {
                        Err(ResonateError::NetworkError(Box::new(String::from("Could not parse metadata from entry"))))
                    }
                }
                None => Err(ResonateError::NetworkError(Box::new(String::from("Could not collect metadata from ID"))))
            }
        }
        Err(_) => Err(ResonateError::NetworkError(Box::new(String::from("Could not collect metadata from ID"))))
    }
}
