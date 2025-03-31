use std::path::PathBuf;

use crate::backend::error::ResonateError;
use crate::frontend::message::Message;

use crate::backend::web::{collect_metadata, flatsearch};
use crate::backend::util::{desync, AM};
use crate::backend::database::Database;

pub async fn async_flatsearch(executable_dir: PathBuf, query: String) -> Message {
    match flatsearch(executable_dir, &query).await {
        Ok(results) => Message::LoadSearchResults(results),
        Err(_) => Message::DLPWarning
    }
}

pub async fn async_populate(executable_dir: Option<PathBuf>, music_dir: PathBuf, thumbnail_dir: PathBuf, id: String, database: AM<Database>) -> Message {
    match collect_metadata(match executable_dir {
        Some(pathbuf) => Some(pathbuf.as_path()),
        None => None
    }, music_dir.as_path(), thumbnail_dir.as_path(), &id).await {
        Ok(song) => {
            let database = desync(&database);
            database.emplace_song_and_record_id(&song, true);
        }
        Err(_) => return Message::None
    }
}
