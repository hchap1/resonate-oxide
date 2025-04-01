use std::time::Duration;
use std::thread::sleep;

use iced::widget::Column;
use iced::widget::Container;
use iced::widget::Row;
use iced::widget::TextInput;
use iced::Task;
use iced::Element;

use crate::backend::filemanager::DataDir;
use crate::frontend::message::Message;
use crate::frontend::application::Page;
use crate::frontend::backend_interface::async_flatsearch;
use crate::frontend::backend_interface::AsyncMetadataCollectionPool;

use crate::backend::util::{consume, AM};
use crate::backend::database::Database;
use crate::backend::music::Song;

use super::widgets::ResonateWidget;

pub struct SearchPage {
    query: String,
    directories: DataDir,
    database: AM<Database>,
    search_results: Option<Vec<Song>>
}

impl SearchPage {
    pub fn new(directories: DataDir, database: AM<Database>) -> Self {
        Self {
            query: String::new(),
            directories,
            database,
            search_results: None
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

        let mut column = Column::new()
            .push(header);

        match self.search_results.as_ref() {
            Some(search_results) => {
                for song in search_results {
                    column = column.push(ResonateWidget::search_result(song));
                }
            }
            None => {}
        }

        Container::new(column).into()
    }

    fn update(self: &mut Self, message: Message) -> Task<Message> {
        match message {
            Message::TextInput(new_value) => { self.query = new_value; Task::none() }
            Message::SubmitSearch => {
                let dlp_path = match self.directories.get_dlp_ref() {
                    Some(dlp_path) => dlp_path.to_path_buf(),
                    None => return Task::none()
                };
                println!("DLP-PATH located, future spawned, consuming: {}", self.query);
                Task::<Message>::future(async_flatsearch(dlp_path, consume(&mut self.query)))
            }

            Message::LoadSearchResults(search_results) => {
                Task::stream(AsyncMetadataCollectionPool::new(
                    search_results,
                    match self.directories.get_dlp_ref() {
                        Some(dlp_ref) => Some(dlp_ref.to_path_buf()),
                        None => None
                    },
                    self.directories.get_music_ref().to_path_buf(),
                    self.directories.get_thumbnails_ref().to_path_buf(),
                    self.database.clone()
                ))
            }

            Message::SearchResult(song) => {
                println!("Search results returned for {}", song.title);
                match self.search_results.as_mut() {
                    Some(search_results) => { search_results.push(song); Task::none() }
                    None => { self.search_results = Some(vec![song]); Task::none() }
                }
            }

            Message::MultiSearchResult(songs) => {
                println!("Received songs!");
                Task::batch(songs.into_iter().map(|song| Task::done(Message::SearchResult(song))))
            }

            _ => Task::none()
        }
    }
}
