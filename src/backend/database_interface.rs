use std::path::PathBuf;

use crate::backend::database_manager::Database;
use crate::backend::database_manager::DatabaseParam;
use crate::backend::database_manager::DatabaseParams;
use crate::backend::sql::*;
use crate::backend::music::Song;

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

    /// Get song by sql ID (not youtube ID)
    pub async fn get_song_by_id(
        database: &Database, song_id: usize, music_path: PathBuf, thumbnail_path: PathBuf
    ) -> Option<Song> {
        let query = match database.query_map(
            SELECT_SONG_BY_SQL_ID, DatabaseParams::single(DatabaseParam::Usize(song_id))
        ).await {
            Ok(results) => results,
            Err(_) => return None
        };

        let mut songs = Vec::new();
        for row in query {
            let music_path = music_path.clone();
            let thumbnail_path = thumbnail_path.clone();

            if let Some(song) = tokio::task::spawn_blocking(move || 
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
            ).await.ok() {
                songs.push(song)
            }
        }

        songs.pop()
    }
}
