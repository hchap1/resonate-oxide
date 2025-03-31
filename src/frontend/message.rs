#[derive(Clone, Debug)]
pub enum Message {
    None,                           // Empty message for map task
    LoadPage(PageType),             // Loads a new page based on the PageType enum
    TextInput(String),              // Primary TextInput task for single-entry pages
    SubmitSearch,                   // Primary task for single-entry-pages
    LoadSearchResults(Vec<String>), // Create a batch of tasks to pull down metadata for each ID and queue into search results buffer
    DLPWarning,                     // Notify the user that the current action requires yt-dlp.
    CollectMetadata(String)         // Task created by LoadSearchResults
}

#[derive(Clone, Debug)]
pub enum PageType {
    SearchSongs
}
