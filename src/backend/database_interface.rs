use std::path::PathBuf;
use async_channel::Receiver;

use crate::backend::database_manager::DataLink;
use crate::backend::database_manager::DatabaseParam;
use crate::backend::database_manager::DatabaseParams;
use crate::backend::sql::*;
use crate::backend::music::Song;
use crate::backend::database_manager::ItemStream;
use crate::backend::music::Playlist;
use crate::backend::settings::Secret;

pub struct DatabaseInterface;
impl DatabaseInterface {
    
    /// Create the tables
    pub async fn create_tables(database: DataLink) {
        let _ = database.execute(CREATE_SONG_TABLE, DatabaseParams::empty());
        let _ = database.execute(CREATE_PLAYLIST_TABLE, DatabaseParams::empty());
        let _ = database.execute(CREATE_PLAYLIST_ENTRIES_TABLE, DatabaseParams::empty());
        let _ = database.execute(CREATE_SECRETS_TABLE, DatabaseParams::empty());
    }

    /// Remove song from playlist given song id and playlist id
    pub fn remove_song_from_playlist(database: DataLink, song_id: usize, playlist_id: usize) {
        let _ = database.execute(REMOVE_SONG_FROM_PLAYLIST, DatabaseParams::new(vec![
            DatabaseParam::Usize(song_id), DatabaseParam::Usize(playlist_id)
        ]));
    }

    /// Delete a playlist by id
    pub fn delete_playlist(database: DataLink, playlist_id: usize) {
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

    pub fn construct_playlist(
        row: Vec<DatabaseParam>
    ) -> Option<Playlist> {
        if row.len() != 2 {
            None
        } else {
            Some(Playlist { id: row[0].usize(), name: row[1].string() })
        }
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

    /// Yield a receiver where database results will come through
    pub fn select_all_songs( database: DataLink) -> Receiver<ItemStream> {
        database.query_stream(SELECT_ALL_SONGS, DatabaseParams::empty())
    }

    /// Inserts a song into the database, returning the new ID of the song.
    /// Relies on implementation to make sure there are no duplicates
    pub async fn insert_song(
        database: DataLink, mut song: Song
    ) -> Option<usize> {
        database.insert(INSERT_SONG, DatabaseParams::new(vec![
            DatabaseParam::String(song.yt_id),
            DatabaseParam::String(song.title),
            DatabaseParam::String(song.artist),
            DatabaseParam::String(song.album.take().unwrap_or(String::from("none"))),
            DatabaseParam::Usize(song.duration.as_secs() as usize)
        ])).await
    }

    /// Inserts a playlist into the database, returning the new ID of said playlist.
    /// Does not check for duplicates.
    pub async fn insert_playlist(
        database: DataLink, mut playlist: Playlist
    ) -> Playlist {
        if let Some(id) = database.insert(
            INSERT_PLAYLIST, DatabaseParams::single(DatabaseParam::String(playlist.name.clone()))
        ).await { playlist.id = id; };
        playlist
    }

    /// Change the name of a playlist
    pub fn update_playlist_name(database: DataLink, playlist: Playlist) {
        let _ = database.execute(UPDATE_PLAYLIST_NAME, DatabaseParams::new(vec![
            DatabaseParam::Usize(playlist.id),
            DatabaseParam::String(playlist.name.clone())
        ]));
    }

    /// Add song to playlist
    pub fn insert_playlist_entry(database: DataLink, song_id: usize, playlist_id: usize) {
        let _ = database.execute(INSERT_ENTRY, DatabaseParams::new(vec![
            DatabaseParam::Usize(playlist_id),
            DatabaseParam::Usize(song_id)
        ]));
    }

    /// Get playlist by ID, if it exists
    pub async fn get_playlist_by_id(
        database: DataLink, playlist_id: usize
    ) -> Option<Playlist> {
        let rows = match database.query_map(
            SELECT_PLAYLIST_BY_ID, DatabaseParams::single(DatabaseParam::Usize(playlist_id))
        ).await {
            Ok(rows) => rows,
            Err(_) => return None
        };
        rows.into_iter().filter_map(|row| {
            Self::construct_playlist(row)
        }).collect::<Vec<Playlist>>().pop()
    }

    /// Stream every playlist
    pub fn select_all_playlists(database: DataLink) -> Receiver<ItemStream> {
        println!("Selecting all playlists");
        database.query_stream(SELECT_ALL_PLAYLISTS, DatabaseParams::empty())
    }

    /// Stream all songs in a playlist
    pub fn select_all_songs_in_playlist(database: DataLink, playlist_id: usize) -> Receiver<ItemStream> {
        database.query_stream(SELECT_ALL_SONGS_IN_PLAYLIST, DatabaseParams::single(DatabaseParam::Usize(playlist_id)))
    }

    /// Exact string matching
    pub async fn select_song_by_title(
        database: DataLink, title: String, music_path: PathBuf, thumbnail_path: PathBuf
    ) -> Option<Song> {
        let rows = match database.query_map(
            SELECT_SONG_BY_TITLE, DatabaseParams::single(DatabaseParam::String(title))
        ).await {
            Ok(rows) => rows,
            Err(_) => return None
        };
        Self::construct_songs(rows, music_path, thumbnail_path).await.pop()
    }

    /// Batch load secrets
    pub async fn select_multiple_secrets(
        database: DataLink, secrets: Vec<String>
    ) -> Vec<Option<Secret>> {
        let mut compiled: Vec<Option<Secret>> = Vec::new();
        for secret in secrets.into_iter() {
            compiled.push(Self::select_secret_by_name(database.clone(), secret.clone()).await)
        }
        compiled
    }

    /// Get a Secret by the exact name String
    pub async fn select_secret_by_name(
        database: DataLink, name: String
    ) -> Option<Secret> {
        let rows = match database.query_map(
            SELECT_SECRET_BY_NAME, DatabaseParams::single(DatabaseParam::String(name))
        ).await {
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
                    "FM_SESSION" => Secret::FMSession(row[2].string()),
                    _ => return None
                })
            }
        }).collect::<Vec<Secret>>().pop()
    }

    /// Insert a secret, removing the orginal one if it already existed
    pub async fn insert_or_update_secret(database: DataLink, name: String, value: String) -> Result<(), ()> {
        let _ = database.execute_and_wait(
            REMOVE_SECRET_BY_NAME, DatabaseParams::single(DatabaseParam::String(name.clone()))
        ).await;

        database.execute_and_wait(
            INSERT_SECRET, DatabaseParams::new(vec![
                DatabaseParam::String(name),
                DatabaseParam::String(value)
            ])
        ).await
    }

    pub fn blocking_is_unique(database: DataLink, id: String) -> bool {
        let handle = database.query_stream(
            SELECT_SONG_BY_YOUTUBE_ID, DatabaseParams::single(DatabaseParam::String(id))
        );

        let mut unique = true;

        while let Ok(item_stream) = handle.recv_blocking() { match item_stream {
                ItemStream::End => break,
                ItemStream::Error => break,
                ItemStream::Value(_) => unique = false
            }
        }

        unique
    }

    pub fn blocking_insert_song(database: DataLink, mut song: Song) -> Result<usize, ()> {
        match database.insert_stream(INSERT_SONG, DatabaseParams::new(vec![
            DatabaseParam::String(song.yt_id),
            DatabaseParam::String(song.title),
            DatabaseParam::String(song.artist),
            DatabaseParam::String(song.album.take().unwrap_or(String::from("none"))),
            DatabaseParam::Usize(song.duration.as_secs() as usize)
        ])).recv_blocking() {
            Ok(res) => match res {
                super::database_manager::InsertMessage::Success(id) => Ok(id),
                super::database_manager::InsertMessage::Error => Err(())
            },
            Err(_) => Err(())
        }
    }
}
