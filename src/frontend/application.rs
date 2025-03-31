use iced::Element;
use iced::widget::text;
use iced::Task;

use crate::frontend::search_page::SearchPage;
use crate::frontend::message::Message;
use crate::frontend::message::PageType;

use crate::backend::filemanager::DataDir;

pub trait Page {
    fn update(self: &mut Self, message: Message) -> Task<Message>;
    fn view(self: &Self) -> Element<'static, Message>;
}

pub struct Application {
    page: Option<Box<dyn Page>>,
    directories: DataDir
}

impl Default for Application {
    fn default() -> Self {
        let directories = match DataDir::create_or_load() {
            Ok(directories) => directories,
            Err(e) => panic!("{:?}", e)
        };

        let mut application = Self {
            page: None,
            directories
        };

        application.load_page(PageType::SearchSongs);
        application
    }
}

impl Application {
    pub fn view(&self) -> Element<Message> {
        match self.page.as_ref() {
            Some(page) => page.view(),
            None => text("404 - No page.").into()
        }
    }
    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::LoadPage(page_type) => { self.load_page(page_type); Task::none() },
            other => match self.page.as_mut() {
                Some(page) => page.update(other),
                None => Task::none()
            }
        }
    }

    fn load_page(& mut self, page_type: PageType) {
        self.page = Some(Box::new(match page_type {
            PageType::SearchSongs => SearchPage::new(self.directories.ref_package())
        }));
    }
}
