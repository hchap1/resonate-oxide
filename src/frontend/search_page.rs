use iced::widget::Container;
use iced::widget::Row;
use iced::widget::TextInput;
use iced::Task;
use iced::Element;

use crate::frontend::message::Message;
use crate::frontend::application::Page;

use crate::backend::web::flatsearch;
use crate::backend::filemanager::DataDir;

#[derive(Clone)]
pub struct SearchPage<'a> {
    query: String,
    directories: &'a DataDir
}

impl<'a> SearchPage<'a> {
    pub fn new(directories: &'a DataDir) -> Self {
        Self {
            query: String::new(),
            directories
        }
    }
}

impl Page for SearchPage<'_> {
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
                flatsearch(executable_path, music_path, thumbnail_path, query, database)
            }
            _ => {}
        }
    }
}
