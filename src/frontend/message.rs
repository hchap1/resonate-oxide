use std::path::PathBuf;

use rspotify::model::{FullTrack, PlaylistItem};
use rspotify::ClientCredsSpotify;
use rust_fm::auth::WebOAuth;

use crate::backend::settings::Secret;

use crate::backend::audio::{AudioTask, ProgressUpdate, QueueFramework, ScrobbleRequest};
use crate::backend::music::Song;
use crate::backend::rpc::RPCMessage;

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum Message {
    None,                                // Empty message for map task
    LoadPage(PageType, Option<usize>),   // Loads a new page based on the PageType enum
    TextInput(String),                   // Primary TextInput task for single-entry pages
    SubmitSearch,                        // Primary task for single-entry-pages
    LoadSearchResults(Vec<String>),      // Create a batch of tasks to pull down metadata for each ID and queue into search results buffer
    DLPWarning,                          // Notify the user that the current action requires yt-dlp.
    CollectMetadata(String),             // Task created by LoadSearchResults
    SearchResult(Song, bool),            // Final task in the search process - actually adds a finished song to the buffer
    MultiSearchResult(Vec<Song>, bool),        // ^ Option extension allowing for parallel metadata collection in batches or all at once
    UpdateThumbnails,                    // On any page that contains thumbnails, update them
    Download(Song),                      // Download a song asynchronously. Relies on the frontend to manage concurrency
    DownloadAll(Vec<Song>),              // Downloads every single song
    SongDownloaded(Song),
    CreatePlaylist,                      // Create a new "My Playlist" name playlist, adding a number if multiple exist
    StartEditing(usize),                 // Edit the name of a playlist on the Playlists page
    StopEditing,                         // Exit exit mode
    DownloadDLP,                         // Spawns a task to check if DLP is downloaded, and if it isn't, download it
    DLPDownloaded(Option<PathBuf>),      // <- Obvious
    DownloadFailed(Song),

    AddSongToPlaylist(Song, usize),      // This also downloads the song
    SongAddedToPlaylist(usize),          // For updating the GUI
    RemoveSongFromPlaylist(usize, usize),// Song id, playlist id
    DeletePlaylist(usize),
    
    AudioTask(AudioTask),
    QueueUpdate(QueueFramework),         // Queue change
    ProgressUpdate(ProgressUpdate),
    LoadAudio,
    LoadEntirePlaylist(usize, bool),     // Id, whether to shuffle

    RemoveSearchStatus,

    SpotifyCreds(Option<String>, Option<String>),
    SpotifyAuth(Result<ClientCredsSpotify, ()>),
    SpotifyPlaylist(String),
    SpotifyPlaylistItem(PlaylistItem),
    SpotifySongToYoutube(FullTrack),    // Search youtube to find the id of this song before pushing to stack
    SpotifyAuthFailed,
    ClearNotification,
    SavePlaylist,
    SpotifyPlaylistName(String, usize),
    SpotifyInvalidID,

    SpotifyAuthenticationSuccess,
    SpotifyAuthenticationFailedAgain,

    LoadSecrets,
    ChangeSecret(Secret),
    SaveSecret(Secret),
    SubmitSecrets,

    FMAuthenticate,
    FMGetSession(WebOAuth),
    FMAuthFailed(Option<WebOAuth>),
    FMAuthSuccess(WebOAuth),
    FMSaveSecrets,

    FMSetNowPlaying(Song),
    FMPushScrobble(Song),
    FMScrobbleSuccess,
    FMScrobbleFailure,
    ScrobbleRequest(ScrobbleRequest),

    RPCMessage(RPCMessage),

    Hover(usize, bool),

    AllSongsInPlaylist(Vec<Song>, bool),
}

#[derive(Clone, Debug)]
pub enum PageType {
    SearchSongs,
    Playlists,
    ViewPlaylist,
    ImportSpotify,
    Settings
}
