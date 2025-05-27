use std::collections::HashSet;

use iced::alignment::Vertical;
use iced::widget::Column;
use iced::task::Handle;
use iced::widget::Row;
use iced::Length;
use iced::Task;
use iced::Element;

use crate::backend::music::Playlist;
use crate::frontend::message::Message;
use crate::frontend::application::Page;
use crate::frontend::backend_interface::async_flatsearch;
use crate::frontend::backend_interface::AsyncMetadataCollectionPool;
use crate::frontend::backend_interface::DatabaseSearchQuery;

use crate::backend::util::{consume, desync, AM};
use crate::backend::filemanager::DataDir;
use crate::backend::database::Database;
use crate::backend::music::Song;

use super::message::PageType;
use super::widgets::ResonateWidget;

pub struct SearchPage {
    query: String,
    directories: DataDir,
    database: AM<Database>,
    search_results: Option<Vec<Song>>,
    search_handles: Vec<Handle>,
    playlist: Option<Playlist>,

    existing_songs: HashSet<usize>
}

impl SearchPage {
    pub fn new(directories: DataDir, database: AM<Database>, playlist_id: usize) -> Self {
        let songs = desync(&database).retrieve_all_songs(directories.get_music_ref(), directories.get_thumbnails_ref());
        let playlist = desync(&database).get_playlist_by_id(playlist_id);

        let songs_in_playlist = match desync(&database).search_playlist(playlist_id, String::new(),
            directories.get_music_ref(), directories.get_thumbnails_ref()) {
            Ok(songs) => songs,
            Err(_) => Vec::new()
        };

        let mut existing_songs: HashSet<usize> = HashSet::new();

        for song in songs_in_playlist {
            existing_songs.insert(song.id);
        }

        Self {
            query: String::new(),
            directories,
            database,
            search_results: Some(songs),
            search_handles: Vec::new(),
            playlist,
            existing_songs
        }
    }
}

impl Page for SearchPage {
    fn view(&self, current_song_downloads: &HashSet<String>) -> Element<'_, Message> {
        let search_bar = Row::new()
            .push(
                ResonateWidget::search_bar("Search...", &self.query)
                    .on_input(Message::TextInput)
                    .on_submit(Message::SubmitSearch)
            );

        let mut column = Column::new().spacing(20);

        if let Some(search_results) = self.search_results.as_ref() {
            for song in search_results {

                if self.existing_songs.contains(&song.id) {
                    continue;
                }

                let is_downloading = current_song_downloads.contains(&song.yt_id);
                column = column.push(
                    ResonateWidget::song(song, self.directories.get_default_thumbnail(), is_downloading)
                        .on_press(Message::AddSongToPlaylist(song.clone(), match self.playlist.as_ref() {
                            Some(playlist) => playlist.id,
                            None => 0
                        })
                    )
                )
            }
        }

        let view_window = ResonateWidget::padded_scrollable(column.into());

        ResonateWidget::window(
            Column::new().spacing(20)
                .push(Row::new().push(ResonateWidget::header(
                    match self.playlist.as_ref() {
                        Some(playlist) => &playlist.name,
                        None => "Searh"
                    }
                )).push(
                    ResonateWidget::button_widget(crate::frontend::assets::home_icon()).on_press(Message::LoadPage(PageType::Playlists, None))
                    ).spacing(20).align_y(Vertical::Center).width(Length::Fill))
                .push(view_window)
                .push(search_bar)
                .into()
        )
    }

    fn update(self: &mut Self, message: Message) -> Task<Message> {
        match message {
            Message::TextInput(new_value) => { self.query = new_value; Task::none() }

            Message::SubmitSearch => {

                if let Some(search_results) = self.search_results.as_mut() { search_results.clear(); }
                for handle in &self.search_handles { handle.abort(); }

                let dlp_path = match self.directories.get_dlp_ref() {
                    Some(dlp_path) => dlp_path.to_path_buf(),
                    None => return Task::none()
                };

                let database = self.database.clone();
                let music_path = self.directories.get_music_ref().to_path_buf();
                let thumbnail_path = self.directories.get_thumbnails_ref().to_path_buf();

                let (flatsearch_task, flatsearch_handle) = Task::<Message>::future(async_flatsearch(dlp_path, self.query.clone())).abortable();
                self.search_handles.push(flatsearch_handle);

                Task::<Message>::stream(DatabaseSearchQuery::new(database, music_path, thumbnail_path, consume(&mut self.query))).chain(
                    flatsearch_task
                )
            }

            Message::LoadSearchResults(search_results) => {
                let ids = match search_results.len() > 3 {
                    true => search_results[0..3].to_vec(),
                    false => search_results
                };
                let (metadata_collector, metadata_collection_handle) = Task::stream(AsyncMetadataCollectionPool::new(
                    ids,
                    match self.directories.get_dlp_ref() {
                        Some(dlp_ref) => Some(dlp_ref.to_path_buf()),
                        None => None
                    },
                    self.directories.get_music_ref().to_path_buf(),
                    self.directories.get_thumbnails_ref().to_path_buf(),
                    self.database.clone()
                )).abortable();

                self.search_handles.push(metadata_collection_handle);
                metadata_collector
            }

            Message::SearchResult(song) => {
                match self.search_results.as_mut() {
                    Some(search_results) => search_results.push(song),
                    None => self.search_results = Some(vec![song])
                }
                Task::none()
            }

            Message::UpdateThumbnails => {
                if let Some(search_results) = self.search_results.as_mut() {
                    search_results.iter_mut().for_each(|song| song.load_paths(self.directories.get_music_ref(), self.directories.get_thumbnails_ref()));
                }
                Task::none()
            }

            Message::SongAddedToPlaylist(song_id) => {
                self.existing_songs.insert(song_id);
                Task::none()
            }

            _ => Task::none()
        }
    }
}
