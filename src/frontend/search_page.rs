use iced::widget::Column;
use iced::task::Handle;
use iced::widget::Row;
use iced::widget::TextInput;
use iced::Task;
use iced::Element;

use std::time::Duration;

use crate::frontend::message::Message;
use crate::frontend::application::Page;
use crate::frontend::backend_interface::async_flatsearch;
use crate::frontend::backend_interface::AsyncMetadataCollectionPool;
use crate::frontend::backend_interface::DatabaseSearchQuery;

use crate::backend::util::{consume, AM};
use crate::backend::filemanager::DataDir;
use crate::backend::database::Database;
use crate::backend::music::Song;

use super::widgets::ResonateWidget;

pub struct SearchPage {
    query: String,
    directories: DataDir,
    database: AM<Database>,
    search_results: Option<Vec<Song>>,
    search_handles: Vec<Handle>
}

impl SearchPage {
    pub fn new(directories: DataDir, database: AM<Database>) -> Self {
        Self {
            query: String::new(),
            directories,
            database,
            // search_results: None,
            search_results: Some(vec![Song::new(
                0,
                String::from("TESTID"),
                String::from("TestSong"),
                String::from("Harrison"),
                Some(String::from("TestSongs")),
                Duration::from_secs(10),
                None,
                None
            )]),
            search_handles: Vec::new()
        }
    }
}

impl Page for SearchPage {
    fn view(&self) -> Element<'_, Message> {
        let header = Row::new()
            .push(
                TextInput::new("Search...", &self.query)
                    .on_input(Message::TextInput)
                    .on_submit(Message::SubmitSearch)
            );

        let mut column = Column::new().spacing(20);

        if let Some(search_results) = self.search_results.as_ref() {
            for song in search_results {
                column = column.push(
                    ResonateWidget::search_result(song)
                )
            }
        }

        let view_window = ResonateWidget::padded_scrollable(column.into());

        ResonateWidget::window(
            Column::new()
                .push(header)
                .push(view_window)
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
                    Some(search_results) => { search_results.push(song); Task::none() }
                    None => { self.search_results = Some(vec![song]); Task::none() }
                }
            }

            Message::MultiSearchResult(songs) => {
                Task::batch(songs.into_iter().map(|song| Task::done(Message::SearchResult(song))))
            }

            _ => Task::none()
        }
    }
}
