use iced::widget::Container;
use iced::widget::Row;
use iced::widget::TextInput;
use iced::Task;
use iced::Element;

use crate::frontend::message::Message;
use crate::frontend::application::Page;
use crate::frontend::backend_interface::async_flatsearch;

use crate::backend::util::{consume, AM};
use crate::backend::filemanager::RefPackage;
use crate::backend::database::Database;

pub struct SearchPage<'a> {
    query: String,
    directories: RefPackage<'a>,
    database: AM<Database>
}

impl<'a> SearchPage<'a> {
    pub fn new(directories: RefPackage<'a>, database: AM<Database>) -> Self {
        Self {
            query: String::new(),
            directories,
            database
        }
    }
}

impl<'a> Page for SearchPage<'a> {
    fn view(self: &Self) -> Element<'static, Message> {
        let header = Row::new()
            .push(
                TextInput::new("Search...", self.query.as_str())
                    .on_input(Message::TextInput)
                    .on_submit(Message::SubmitSearch)
            );

        Container::new(header).into()
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
                search_results.into_iter().map(|result| Task::<Message>::future(async_populate)
            }
            _ => Task::none()
        }
    }
}
