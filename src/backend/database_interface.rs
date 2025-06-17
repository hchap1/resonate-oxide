use std::path::PathBuf;

use strsim::levenshtein;

use crossbeam_channel::Receiver;

use crate::backend::database_manager::Database;
use crate::backend::database_manager::DatabaseParam;
use crate::backend::database_manager::DatabaseParams;
use crate::backend::sql::*;
use crate::backend::music::Song;
use crate::backend::database_manager::ItemStream;
use crate::backend::error::ResonateError;

pub struct DatabaseInterface;
impl DatabaseInterface {
    
    /// Create the tables
    pub fn create_tables(database: &Database) {
        let _ = database.execute(CREATE_SONG_TABLE, DatabaseParams::empty());
        let _ = database.execute(CREATE_PLAYLIST_TABLE, DatabaseParams::empty());
        let _ = database.execute(CREATE_PLAYLIST_ENTRIES_TABLE, DatabaseParams::empty());
        let _ = database.execute(CREATE_SECRETS_TABLE, DatabaseParams::empty());
    }

    /// Remove song from playlist given song id and playlist id
    pub fn remove_song_from_playlist(database: &Database, song_id: usize, playlist_id: usize) {
        let _ = database.execute(REMOVE_SONG_FROM_PLAYLIST, DatabaseParams::new(vec![
            DatabaseParam::Usize(song_id), DatabaseParam::Usize(playlist_id)
        ]));
    }

    /// Delete a playlist by id
    pub fn delete_playlist(database: &Database, playlist_id: usize) {
        let _ = database.execute(REMOVE_ALL_FROM_PLAYLIST, DatabaseParams::single(DatabaseParam::Usize(playlist_id)));
        let _ = database.execute(REMOVE_PLAYLIST, DatabaseParams::single(DatabaseParam::Usize(playlist_id)));
    }

    /// Make song from a single row
    pub async fn construct_song(
        row: Vec<DatabaseParam>, music_path: PathBuf, thumbnail_path: PathBuf
    ) -> Option<Song> {
        if row.len() != 6 {
            return None;
        }

        tokio::task::spawn_blocking(move || 
            Song::load(
                row[0].usize(),
                row[1].string(),
                row[2].string(),
                row[3].string(),
                Some(row[4].string()),
                std::time::Duration::from_secs(row[5].usize() as u64),
                music_path,
                thumbnail_path
            )
        ).await.ok()
    }

    /// Make a song out of the rows
    pub async fn construct_songs(
        rows: Vec<Vec<DatabaseParam>>, music_path: PathBuf, thumbnail_path: PathBuf
    ) -> Vec<Song> {
        let mut songs = Vec::new();
        for row in rows {
            if let Some(song) = Self::construct_song(row, music_path.clone(), thumbnail_path.clone()).await {
                songs.push(song);
            }
        }
        songs
    }

    /// Get song by sql ID (not youtube ID)
    pub async fn get_song_by_id(
        database: &Database, song_id: usize, music_path: PathBuf, thumbnail_path: PathBuf
    ) -> Option<Song> {
        let results = match database.query_map(
            SELECT_SONG_BY_SQL_ID, DatabaseParams::single(DatabaseParam::Usize(song_id))
        ).await {
            Ok(results) => results,
            Err(_) => return None
        };

        Self::construct_songs(results, music_path, thumbnail_path).await.pop()
    }

    /// Yield a receiver where database results will come through
    pub fn select_all_songs( database: &Database) -> Receiver<ItemStream> {
        database.query_stream(SELECT_ALL_SONGS, DatabaseParams::empty())
    }

    /// Select song by youtube id
    pub async fn select_song_by_youtube_id(
        database: &Database, youtube_id: String, music_path: PathBuf, thumbnail_path: PathBuf
    ) -> Option<Song> {
        let results = match database.query_map(
            SELECT_SONG_BY_YOUTUBE_ID, DatabaseParams::single(DatabaseParam::String(youtube_id))
        ).await {
            Ok(results) => results,
            Err(_) => return None
        };

        Self::construct_songs(results, music_path, thumbnail_path).await.pop()
    }

    /// Returns whether a song exists, or Err if it could not be accessed
    pub async fn is_unique(
        database: &Database, youtube_id: String, music_path: PathBuf, thumbnail_path: PathBuf
    ) -> bool {
        Self::select_song_by_youtube_id(database, youtube_id, music_path, thumbnail_path).await.is_none()
    }

    /// Inserts a song into the database, returning the new ID of the song.
    /// Will check and make sure it does not already exist, based on yt-id
    /// If it already exists, it will still return Ok(id)
    pub async fn insert_song(
        database: &Database, mut song: Song, music_path: PathBuf, thumbnail_path: PathBuf
    ) -> Option<usize> {
        let existing_song = Self::select_song_by_youtube_id(
            &database, song.yt_id.clone(), music_path, thumbnail_path
        ).await;

        if let Some(song) = existing_song { return Some(song.id) };

        database.insert(INSERT_SONG, DatabaseParams::new(vec![
            DatabaseParam::String(song.yt_id),
            DatabaseParam::String(song.title),
            DatabaseParam::String(song.artist),
            DatabaseParam::String(song.album.take().unwrap_or(String::from("none"))),
            DatabaseParam::Usize(song.duration.as_secs() as usize)
        ])).await
    }
}
