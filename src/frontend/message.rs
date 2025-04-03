use crate::backend::music::Song;

#[derive(Clone, Debug)]
pub enum Message {
    None,                           // Empty message for map task
    LoadPage(PageType),             // Loads a new page based on the PageType enum
    TextInput(String),              // Primary TextInput task for single-entry pages
    SubmitSearch,                   // Primary task for single-entry-pages
    LoadSearchResults(Vec<String>), // Create a batch of tasks to pull down metadata for each ID and queue into search results buffer
    DLPWarning,                     // Notify the user that the current action requires yt-dlp.
    CollectMetadata(String),        // Task created by LoadSearchResults
    SearchResult(Song),             // Final task in the search process - actually adds a finished song to the buffer
    MultiSearchResult(Vec<Song>),   // ^ Option extension allowing for parallel metadata collection in batches or all at once
    UpdateThumbnails,               // On any page that contains thumbnails, update them
    Download(String)                // Download a song asynchronously. Relies on the frontend to manage concurrency
}

#[derive(Clone, Debug)]
pub enum PageType {
    SearchSongs
}
