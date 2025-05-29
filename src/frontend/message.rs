use std::path::PathBuf;

use crate::backend::audio::{AudioTask, QueueFramework};
use crate::backend::music::Song;

#[derive(Clone, Debug)]
pub enum Message {
    None,                                // Empty message for map task
    LoadPage(PageType, Option<usize>),   // Loads a new page based on the PageType enum
    TextInput(String),                   // Primary TextInput task for single-entry pages
    SubmitSearch,                        // Primary task for single-entry-pages
    LoadSearchResults(Vec<String>),      // Create a batch of tasks to pull down metadata for each ID and queue into search results buffer
    DLPWarning,                          // Notify the user that the current action requires yt-dlp.
    CollectMetadata(String),             // Task created by LoadSearchResults
    SearchResult(Song),                  // Final task in the search process - actually adds a finished song to the buffer
    MultiSearchResult(Vec<Song>),        // ^ Option extension allowing for parallel metadata collection in batches or all at once
    UpdateThumbnails,                    // On any page that contains thumbnails, update them
    Download(Song),                      // Download a song asynchronously. Relies on the frontend to manage concurrency
    SongDownloaded(Song),
    CreatePlaylist,                      // Create a new "My Playlist" name playlist, adding a number if multiple exist
    StartEditing(usize),                 // Edit the name of a playlist on the Playlists page
    StopEditing,                         // Exit exit mode
    DownloadDLP,                         // Spawns a task to check if DLP is downloaded, and if it isn't, download it
    DLPDownloaded(Option<PathBuf>),      // <- Obvious

    AddSongToPlaylist(Song, usize),      // This also downloads the song
    SongAddedToPlaylist(usize),          // For updating the GUI
    RemoveSongFromPlaylist(usize, usize),// Song id, playlist id
    DeletePlaylist(usize),
    
    AudioTask(AudioTask),
    QueueUpdate(QueueFramework),         // Queue change
    LoadAudio,
    LoadEntirePlaylist(usize, bool),     // Id, whether to shuffle
}

#[derive(Clone, Debug)]
pub enum PageType {
    SearchSongs,
    Playlists,
    ViewPlaylist
}
