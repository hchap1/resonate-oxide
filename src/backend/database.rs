use std::path::Path;
use std::path::PathBuf;

use rusqlite::Connection;

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

    /// Find a place for the song in the database, mutating the song to use the id.
    /// Returns whether the song was actually inserted. May be false if already present.
    pub fn emplace_song_and_record_id(&self, song: &mut Song) -> Result<bool, ResonateError> {

    }
}
