use iced::Element;
use iced::Task;

use crate::frontend::search_page::SearchPage;
use crate::frontend::message::Message;
use crate::frontend::message::PageType;

pub trait Page {
    fn update(self: &mut Self, message: Message) -> Task<Message>;
    fn view(self: &Self) -> Element<'static, Message>;
}

fn create_page(page_type: PageType) -> Box<dyn Page> {
    Box::new(match page_type {
        PageType::SearchSongs => SearchPage::default()
    })
}

pub struct Application {
    page: Box<dyn Page>
}

impl Default for Application {
    fn default() -> Self {
        Self {
            page: Box::new(SearchPage::default())
        }
    }
}

impl Application {
    pub fn view(&self) -> Element<Message> { self.page.view() }
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::LoadPage(page_type) => { self.page = create_page(page_type); Task::none() },
            other => self.page.update(other)
        }
    }
}
