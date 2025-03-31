use std::collections::HashSet;
use std::path::Path;
use std::path::PathBuf;
use std::sync::mpsc::{
    self,
    Sender,
    Receiver
};

use rusqlite::Connection;
use rusqlite::params;

use crate::backend::sql::*;
use crate::backend::error::ResonateError;
use crate::backend::music::Song;

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

    /// Quickly check if a yt-id already exists in the database
    pub fn is_unique(&self, yt_id: &String) -> bool {
        match Query::new(&self.connection).check_if_yt_id_exists().query(params![yt_id]) {
            Ok(mut rows) => if let Ok(row) = rows.next() { row.is_some() } else { false },
            Err(_) => false
        }
    }

    /// Find a place for the song in the database, returning whether a change was made and what id the song takes
    /// Flag whther to check before adding. This is so that mass-creation of songs can be checked externally with a hashset for greater efficiency
    /// Individual songs are checked without initializing a whole hashset.
    pub fn emplace_song_and_record_id(&self, song: &Song, check: bool) -> Result<(bool, usize), ResonateError> {
        // If the song is already in the database, return false
        if check { if !self.is_unique(&song.title) { return Ok((false, 0)); }}
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

    pub async fn add_song_to_playlist(&self, song_id: usize, playlist_id: usize) -> Result<bool, ResonateError> {
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
}
