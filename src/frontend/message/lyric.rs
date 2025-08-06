use crate::backend::music::Song;

#[derive(Debug, Clone)]
pub enum LyricMsg {
    SpawnCollector,
    RequestLyrics(Song),
    ReturnLyrics(String)
}
