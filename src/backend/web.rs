use std::path::PathBuf;
use std::path::Path;
use std::time::Duration;

use image::imageops::FilterType;
use youtube_dl::YoutubeDl;
use tokio::process::Command;

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
                        let duration = match duration.as_u64() {
                            Some(duration) => duration,
                            None => 0u64
                        };
                        Ok(Song::load(0, entry.id, title, artist, entry.album.take(), Duration::from_secs(duration), music_path, thumbnail_path))
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

pub async fn download_song(dlp_path: PathBuf, music_path: PathBuf, id: String) -> Result<PathBuf, ()> {
    let output = music_path.join(format!("{id}.mp3"));
    let ytdlp = YoutubeDl::new(format!("https://music.youtube.com/watch?v={id}"))
        .youtube_dl_path(dlp_path)
        .extra_arg("-o")
        .extra_arg(output.to_string_lossy().to_string())
        .run_async().await;

    match ytdlp {
        Ok(results) => {
            match results.into_single_video() {
                Some(song) => {
                    if song.id == id {
                        Ok(output)
                    } else {
                        Err(())
                    }
                }
                None => Err(())
            }
        }
        Err(_) => Err(())
    }
}
