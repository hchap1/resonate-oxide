use iced::widget::Container;
use iced::widget::Row;
use iced::widget::TextInput;
use iced::Task;
use iced::Element;

use crate::frontend::message::Message;
use crate::frontend::application::Page;

#[derive(Clone, Default)]
pub struct SearchPage {
    query: String
}

impl Page for SearchPage {
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
            Message::TextInput(new_value) => self.query = new_value,
            Message::SubmitSearch => println!("QUERY: {}", self.query),
            _ => {}
        }
        Task::none()
    }
}
