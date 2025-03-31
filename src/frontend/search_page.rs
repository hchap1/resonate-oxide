use iced::widget::Column;
use iced::widget::Container;
use iced::widget::Row;
use iced::widget::TextInput;
use iced::Task;
use iced::Element;

use crate::frontend::message::Message;
use crate::frontend::application::Page;
use crate::frontend::backend_interface::async_flatsearch;
use crate::frontend::backend_interface::async_populate;

use crate::backend::util::{consume, AM};
use crate::backend::filemanager::RefPackage;
use crate::backend::database::Database;
use crate::backend::music::Song;

use super::widgets::ResonateWidget;

pub struct SearchPage<'a> {
    query: String,
    directories: RefPackage<'a>,
    database: AM<Database>,
    search_results: Option<Vec<Song>>
}

impl<'a> SearchPage<'a> {
    pub fn new(directories: RefPackage<'a>, database: AM<Database>) -> Self {
        Self {
            query: String::new(),
            directories,
            database,
            search_results: None
        }
    }
}

impl<'a> Page<'a> for SearchPage<'a> {
    fn view(self: &'a Self) -> Element<'a, Message> {
        let header = Row::new()
            .push(
                TextInput::new("Search...", self.query.as_str())
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

                let dlp_path = match self.directories.dlp_path {
                    Some(dlp_path) => dlp_path.to_path_buf(),
                    None => return Task::none()
                };

                Task::<Message>::future(async_flatsearch(dlp_path, consume(&mut self.query)))
            }
            Message::LoadSearchResults(search_results) => {
                let tasks = search_results.into_iter().map(|result| Task::<Message>::future(
                    async_populate(match &self.directories.dlp_path {
                        Some(path) => Some(path.to_path_buf()),
                        None => None
                    }, self.directories.music.to_path_buf(), self.directories.thumbnails.to_path_buf(),
                    result, self.database.clone())
                )).collect::<Vec<Task<Message>>>();
                Task::<Message>::batch(tasks)
            }
            Message::SearchResult(song) => match self.search_results.as_mut() {
                Some(search_results) => { search_results.push(song); Task::none() }
                None => Task::none()
            }
            _ => Task::none()
        }
    }
}
