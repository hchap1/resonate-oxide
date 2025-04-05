use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::task::Waker;
use std::time::Duration;

use rusqlite::Connection;
use rusqlite::params;

use crate::backend::sql::*;
use crate::backend::util::AM;
use crate::backend::util::desync;
use crate::backend::error::ResonateError;
use crate::backend::music::Song;
use crate::backend::music::Playlist;

pub struct Database {
    connection: Connection,
    parent_directory: PathBuf
}

impl Database {
    pub fn new(root_dir: &Path) -> Result<Self, ResonateError> {
        let db = Self {
            // Automagically creates a database if it does not exist
            connection: match Connection::open(root_dir.join("data.db")) {
                Ok(connection) => connection,
                Err(_) => return Err(ResonateError::DatabaseCreationError)
            },
            // Store parent directory for future reference
            parent_directory: root_dir.to_path_buf()
        };

        // Attempt to create the necessary tables.
        // This shouldn't fail, but is handled either way.
        match db.connection.execute(CREATE_SONG_TABLE,[]) {
            Ok(_) => {},
            Err(_) => return Err(ResonateError::TableCreationError)
        }

        match db.connection.execute(CREATE_PLAYLIST_TABLE,[]) {
            Ok(_) => {},
            Err(_) => return Err(ResonateError::TableCreationError)
        }
        
        match db.connection.execute(CREATE_PLAYLIST_ENTRIES_TABLE,[]) {
            Ok(_) => {},
            Err(_) => return Err(ResonateError::TableCreationError)
        }

        Ok(db)
    }

    /// Attempt to hash every song in the database by YT-ID for uniqueness check
    pub fn hash_all_songs(&self) -> HashSet<String> {
        match Query::new(&self.connection).retrieve_all_songs().query_map(params![], |row| {
            row.get::<_, String>(0)
        }) {
            Ok(results) => HashSet::from_iter(results.filter_map(|potential_id| match potential_id {
                Ok(id) => Some(id),
                Err(_) => None
            })),
            Err(_) => HashSet::<String>::new()
        }
    }

    /// Get songs where the name, string or artist matches a word
    pub fn keyword(&self, music_path: &Path, thumbnail_path: &Path, query: String) -> Vec<Song> {
        let similar_query = format!("%{query}%");
        Query::new(&self.connection).get_song_by_match().query_map(params![similar_query, similar_query, similar_query], |row| {
            let id = row.get::<_, usize>(0).unwrap();
            let yt_id = row.get::<_, String>(1).unwrap();
            let title = row.get::<_, String>(2).unwrap();
            let artist = row.get::<_, String>(3).unwrap();
            let album = row.get::<_, String>(4).unwrap();
            let duration = row.get::<_, usize>(5).unwrap();

            Ok(Song::load(id, yt_id, title, artist, Some(album), Duration::from_secs(duration as u64), music_path, thumbnail_path))
        }).unwrap().filter_map(
            |res| match res {
                Ok(song) => Some(song),
                Err(_) => None
            }
        ).collect()
    }

    /// Retrieve all songs
    pub fn retrieve_all_songs(&self, music_path: &Path, thumbnail_path: &Path) -> Vec<Song> {
        Query::new(&self.connection).retrieve_all_songs().query_map(params![], |row| {
            let id = row.get::<_, usize>(0).unwrap();
            let yt_id = row.get::<_, String>(1).unwrap();
            let title = row.get::<_, String>(2).unwrap();
            let artist = row.get::<_, String>(3).unwrap();
            let album = row.get::<_, String>(4).unwrap();
            let duration = row.get::<_, usize>(5).unwrap();

            Ok(Song::load(id, yt_id, title, artist, Some(album), Duration::from_secs(duration as u64), music_path, thumbnail_path))
        }).unwrap().filter_map(
            |res| match res {
                Ok(song) => Some(song),
                Err(_) => None
            }
        ).collect()
   }

    /// Quickly check if a yt-id already exists in the database
    pub fn is_unique(&self, yt_id: &String) -> bool {
        !Query::new(&self.connection).check_if_yt_id_exists().query_map(params![yt_id], |row| {
            row.get::<_, String>(0)
        }).unwrap().any(|row| match row {
            Ok(id) => id == *yt_id,
            Err(_) => false
        })
    }

    /// Find a place for the song in the database, returning whether a change was made and what id the song takes
    /// Flag whther to check before adding. This is so that mass-creation of songs can be checked externally with a hashset for greater efficiency
    /// Individual songs are checked without initializing a whole hashset.
    pub fn emplace_song_and_record_id(&self, song: &Song, check: bool) -> Result<(bool, usize), ResonateError> {
        // If the song is already in the database, return false
        if check { if !self.is_unique(&song.yt_id) { return Ok((false, 0)); }}
        let album: &String = match song.album.as_ref() {
            Some(album) => album,
            None => &String::new()
        };
        match Query::new(&self.connection).insert_song().execute(params![
            &song.yt_id,
            &song.title,
            &song.artist,
            album,
            &song.duration.as_secs()
        ]) {
            Ok(_) => Ok((true, self.connection.last_insert_rowid() as usize)),
            Err(_) => Err(ResonateError::SQLError)
        }
    }

    pub fn add_song_to_playlist(&self, song_id: usize, playlist_id: usize) -> Result<bool, ResonateError> {
        // If the song is in the playlist already, return false
        if match Query::new(&self.connection).check_if_song_in_playlist().query(params![playlist_id, song_id]) {
            Ok(mut rows) => if let Ok(row) = rows.next() { row.is_some() } else { false }
            Err(_) => return Err(ResonateError::SQLError)
        } {
            return Ok(false);
        }

        // Add the song to the playlist
        match Query::new(&self.connection).add_song_to_playlist().execute(params![
            playlist_id,
            song_id
        ]) {
            Ok(_) => Ok(true),
            Err(_) => Err(ResonateError::SQLError)
        }
    }

    pub fn retrieve_all_playlists(&self) -> Vec<Playlist> {
        match Query::new(&self.connection).get_all_playlists().query_map(params![], |row| {
            Ok(Playlist {
                id: row.get::<_, usize>(0).unwrap(),
                name: row.get::<_, String>(1).unwrap()
            })
        }) {
            Ok(playlists) => playlists.filter_map(|playlist| match playlist {
                Ok(playlist) => Some(playlist),
                Err(_) => None
            }).collect(),
            Err(_) => vec![]
        }
    }
}

pub fn search_mutex(database: AM<Database>, music_path: PathBuf, thumbnail_path: PathBuf, query: String, waker: AM<Option<Waker>>) ->  Vec<Song> {
    let database = desync(&database);
    let database_output = database.keyword(music_path.as_path(), thumbnail_path.as_path(), query);

    let waker = desync(&waker);
    match waker.as_ref() {
        Some(waker) => waker.wake_by_ref(),
        None => {} // Nothing we can do about it.
    }

    database_output
}
