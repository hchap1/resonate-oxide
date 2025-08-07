use std::collections::HashMap;
use std::path::PathBuf;
use std::path::Path;
use std::process::Command;
use std::collections::HashSet;

use image::imageops::FilterType;

use crate::backend::music::Song;

pub struct Thumbnail {
    thumbnail: PathBuf,
    fullsize: PathBuf,
    blurred: PathBuf
}

pub struct ThumbnailManager {
    thumbnails: HashMap<String, Thumbnail>,
    downloading: HashSet<String>
}

impl ThumbnailManager {

    pub fn do_all_exist(song: &mut Song, thumbnail_dir: &Path) -> bool {
        if song.thumbnail_path.is_some() && song.full_image_path.is_some() && song.blurred_image_path.is_some() {
            return true;
        }
        song.load_thumbnail_paths(thumbnail_dir);
        song.thumbnail_path.is_some() && song.full_image_path.is_some() && song.blurred_image_path.is_some()
    }

    pub async fn download_thumbnail(
        dlp_path: PathBuf, thumbnail_dir: PathBuf, mut song: Song
    ) -> Result<PathBuf, ()> {

        let album = match song.album.as_ref() {
            Some(album) => {
                format!("{}.png", album.replace(' ', "_"))
            }
            None => {
                format!("{}.png", song.yt_id.replace(' ', "_"))
            }
        };

        let path = thumbnail_dir.join(&album).to_string_lossy().to_string();

        if Self::do_all_exist(&mut song, thumbnail_dir.as_path()){
            return Err(())
        }

        let mut ytdlp = Command::new(dlp_path);
        ytdlp.arg("--write-thumbnail")
            .arg("--skip-download")
            .arg("--no-check-certificate")
            .arg(format!("https://music.youtube.com/watch?v={}", song.yt_id))
            .arg("-o")
            .arg(path.clone());

        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            ytdlp = ytdlp.creation_flags(0x08000000);
        }

        ytdlp.spawn().unwrap();

        let raw = match image::open(thumbnail_dir.join(format!("{album}.webp"))) {
            Ok(image) => image,
            Err(_) => return Err(())
        };

        let original_width = raw.width();
        let original_height = raw.height();

        // small size
        let new_height = 64;
        let new_width = (original_width as f64 * (new_height as f64 / original_height as f64)) as u32;
        let scaled = raw.resize(new_width, new_height, FilterType::Gaussian);

        let height = scaled.height();
        let padding = (scaled.width() - height) / 2;
        let cropped = scaled.crop_imm(padding, 0, height, height);
        let result = thumbnail_dir.join(format!("{album}.png"));
        let _ = cropped.save(&result);

        // full size
        let size = original_width.min(original_height);
        let x_offset = (original_width - size) / 2;
        let y_offset = (original_height - size) / 2;
        let square_cropped = raw.crop_imm(x_offset, y_offset, size, size);
        let fullsize_path = thumbnail_dir.join(format!("{album}_fullsize.png"));
        let _ = square_cropped.save(&fullsize_path);

        // blurred
        let blurred = square_cropped.blur(25.0);
        let blurred_path = thumbnail_dir.join(format!("{album}_blurred.png"));
        let _ = blurred.save(&blurred_path);

        // delete webp
        let _ = std::fs::remove_file(thumbnail_dir.join(format!("{album}.webp")));

        match result.exists() {
            true => Ok(result),
            false => Err(())
        }
    }
}
