use crate::backend::database_manager::Database;
use crate::backend::database_manager::DatabaseParam;
use crate::backend::database_manager::DatabaseParams;
use crate::backend::sql::*;

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
        let _ = database.execute(DELETE_ALL_SONGS_IN_PLAYLIST, DatabaseParams::new(vec![
            DatabaseParam::Usize(playlist_id)
        ]));
    }
}
