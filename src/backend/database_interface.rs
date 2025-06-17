use std::future::Future;
use std::path::PathBuf;

use crossbeam_channel::Receiver;

use crate::backend::database_manager::Database;
use crate::backend::database_manager::DatabaseParam;
use crate::backend::database_manager::DatabaseParams;
use crate::backend::sql::*;
use crate::backend::music::Song;
use crate::backend::database_manager::ItemStream;
use crate::backend::music::Playlist;
use crate::backend::error::ResonateError;

use super::settings::Secret;

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

    /// Get an async future to produce song by sql ID (not youtube ID)
    pub fn get_song_by_id_handle<'a>(
        database: &'a Database, song_id: usize
    ) -> impl Future<Output = Result<Vec<Vec<DatabaseParam>>, ResonateError>> + 'a {
        database.query_map(SELECT_SONG_BY_SQL_ID, DatabaseParams::single(DatabaseParam::Usize(song_id)))
    }

    /// Yield a receiver where database results will come through
    pub fn select_all_songs( database: &Database) -> Receiver<ItemStream> {
        database.query_stream(SELECT_ALL_SONGS, DatabaseParams::empty())
    }

    /// Request select song by youtube id
    pub fn select_song_by_youtube_id_handle<'a>(
        database: &'a Database, youtube_id: String
    ) -> impl Future<Output = Result<Vec<Vec<DatabaseParam>>, ResonateError>> + 'a {
        database.query_map(SELECT_SONG_BY_YOUTUBE_ID, DatabaseParams::single(DatabaseParam::String(youtube_id)))
    }

    pub async fn select_song_by_youtube_id(
        handle: impl Future<Output = Result<Vec<Vec<DatabaseParam>>, ResonateError>>,
        music_path: PathBuf, thumbnail_path: PathBuf
    ) -> Option<Song> {
        let rows = match handle.await {
            Ok(rows) => rows,
            Err(_) => return None
        };

        Self::construct_songs(rows, music_path, thumbnail_path).await.pop()
    }

    /// Inserts a song into the database, returning the new ID of the song.
    /// Relies on implementation to make sure there are no duplicates
    pub fn insert_song<'a>(
        database: &'a Database, mut song: Song
    ) -> impl Future<Output = Option<usize>> + 'a {
        database.insert(INSERT_SONG, DatabaseParams::new(vec![
            DatabaseParam::String(song.yt_id),
            DatabaseParam::String(song.title),
            DatabaseParam::String(song.artist),
            DatabaseParam::String(song.album.take().unwrap_or(String::from("none"))),
            DatabaseParam::Usize(song.duration.as_secs() as usize)
        ]))
    }

    /// Inserts a playlist into the database, returning the new ID of said playlist.
    /// Does not check for duplicates.
    pub fn insert_playlist<'a>(
        database: &'a Database, playlist: Playlist
    ) -> impl Future<Output = Option<usize>> + 'a {
        database.insert(INSERT_PLAYLIST, DatabaseParams::single(DatabaseParam::String(playlist.name)))
    }

    /// Change the name of a playlist
    pub fn update_playlist_name(database: &Database, playlist: Playlist) {
        let _ = database.execute(UPDATE_PLAYLIST_NAME, DatabaseParams::new(vec![
            DatabaseParam::Usize(playlist.id),
            DatabaseParam::String(playlist.name.clone())
        ]));
    }

    /// Add song to playlist
    pub fn add_song_to_playlist(database: &Database, song_id: usize, playlist_id: usize) {
        let _ = database.execute(INSERT_ENTRY, DatabaseParams::new(vec![
            DatabaseParam::Usize(playlist_id),
            DatabaseParam::Usize(song_id)
        ]));
    }

    /// Produce a handle for a playlist_id query
    pub fn get_playlist_by_id_handle<'a>(
        database: &'a Database, playlist_id: usize
    ) -> impl Future<Output = Result<Vec<Vec<DatabaseParam>>, ResonateError>> + 'a {
        database.query_map(SELECT_PLAYLIST_BY_ID, DatabaseParams::single(DatabaseParam::Usize(playlist_id)))
    }

    /// Get playlist by id
    pub async fn get_playlist_by_id(
        handle: impl Future<Output = Result<Vec<Vec<DatabaseParam>>, ResonateError>>
    ) -> Option<Playlist> {
        let rows = match handle.await {
            Ok(rows) => rows,
            Err(_) => return None
        };

        rows.into_iter().filter_map(|row| {
            if row.len() != 2 {
                None
            } else {
                Some(Playlist { id: row[0].usize(), name: row[1].string() })
            }
        }).collect::<Vec<Playlist>>().pop()
    }

    /// Stream every playlist
    pub fn select_all_playlists(database: &Database) -> Receiver<ItemStream> {
        database.query_stream(SELECT_ALL_PLAYLISTS, DatabaseParams::empty())
    }

    /// Stream all songs in a playlist
    pub fn select_all_songs_in_playlist(database: &Database, playlist_id: usize) -> Receiver<ItemStream> {
        database.query_stream(SELECT_ALL_SONGS_IN_PLAYLIST, DatabaseParams::single(DatabaseParam::Usize(playlist_id)))
    }

    pub fn select_song_by_title_handle<'a>(
        database: &'a Database, title: String
    ) -> impl Future<Output = Result<Vec<Vec<DatabaseParam>>, ResonateError>> + 'a {
        database.query_map(SELECT_SONG_BY_TITLE, DatabaseParams::single(DatabaseParam::String(title)))
    }

    /// Construct a song from a future that yields a row
    pub async fn song_from_handle(
        handle: impl Future<Output = Result<Vec<Vec<DatabaseParam>>, ResonateError>>,
        music_path: PathBuf,
        thumbnail_path: PathBuf
    ) -> Option<Song> {
        let rows = match handle.await {
            Ok(rows) => rows,
            Err(_) => return None
        };

        Self::construct_songs(rows, music_path, thumbnail_path).await.pop()
    }

    /// Expects an ItemStream that will yield every song in a playlist.
    pub async fn get_download_statistics(
        receiver: Receiver<ItemStream>, music_path: PathBuf, thumbnail_path: PathBuf
    ) -> Result<(usize, usize), ResonateError> {

        let mut rows = Vec::new();
        let mut temp_receiver = receiver.clone();

        while let Ok(result) = tokio::task::spawn_blocking(move || temp_receiver.recv()).await {
            let message = match result {
                Ok(result) => result,
                Err(_) => return Err(ResonateError::GenericError)
            };

            match message {
                ItemStream::Value(row) => rows.push(row),
                ItemStream::End => break,
                ItemStream::Error => return Err(ResonateError::GenericError)
            }

            temp_receiver = receiver.clone();
        }

        let songs = Self::construct_songs(rows, music_path, thumbnail_path).await;
        Ok((
            songs.iter().filter(|x| x.music_path.is_some()).count(),
            songs.len()
        ))
    }

    /// Create a handle to a task getting a secret by name
    pub fn select_secret_by_name_handle<'a>(
        database: &'a Database, name: String
    ) -> impl Future<Output = Result<Vec<Vec<DatabaseParam>>, ResonateError>> + 'a {
        database.query_map(SELECT_SECRET_BY_NAME, DatabaseParams::single(DatabaseParam::String(name)))
    }

    /// Await a handle yielding database rows and produce a secret
    pub async fn select_secret_by_name(
        handle: impl Future<Output = Result<Vec<Vec<DatabaseParam>>, ResonateError>>
    ) -> Option<Secret> {
        let rows = match handle.await {
            Ok(rows) => rows,
            Err(_) => return None
        };

        rows.into_iter().filter_map(|row| {
            if row.len() != 3 {
                None
            } else {
                let name = row[1].string();

                Some(match name.as_str() {
                    "SPOTIFY_ID" => Secret::SpotifyID(row[2].string()),
                    "SPOTIFY_SECRET" => Secret::SpotifySecret(row[2].string()),
                    "FM_KEY" => Secret::FMKey(row[2].string()),
                    "FM_SECRET" => Secret::FMSecret(row[2].string()),
                    "FM_SESSION" => Secret::FMSecret(row[2].string()),
                    _ => return None
                })
            }
        }).collect::<Vec<Secret>>().pop()
    }
}
