use iced::Element;
use iced::widget::text;
use iced::Task;

use crate::frontend::search_page::SearchPage;
use crate::frontend::message::Message;
use crate::frontend::message::PageType;

use crate::backend::filemanager::DataDir;
use crate::backend::filemanager::RefPackage;
use crate::backend::database::Database;
use crate::backend::util::{sync, desync, AM};

pub trait Page {
    fn update(self: &mut Self, message: Message) -> Task<Message>;
    fn view(self: &Self) -> Element<'static, Message>;
}

pub struct Application<'a> {
    page: Option<Box<dyn Page + 'a>>,
    directories: &'a DataDir,
    database: AM<Database>
}

impl<'a> Application<'a> {
    pub fn new(directories: &'a DataDir, database: Database) -> Self {
        Self {
            page: Some(Box::new(SearchPage::new(directories.ref_package()))),
            directories,
            database: sync(database)
        }
    }

    pub fn view(&self) -> Element<Message> {
        match self.page.as_ref() {
            Some(page) => page.view(),
            None => text("404 - No page.").into()
        }
    }

    pub fn update(&'a mut self, message: Message) -> Task<Message> {
        match message {
            Message::LoadPage(page_type) => { self.load_page(page_type); Task::none() },
            other => match self.page.as_mut() {
                Some(page) => page.update(other),
                None => Task::none()
            }
        }
    }

    fn load_page(&'a mut self, page_type: PageType) {
        let ref_package: RefPackage = self.directories.ref_package();
        self.page = Some(Box::new(match page_type {
            PageType::SearchSongs => SearchPage::new(ref_package)
        }));
    }
}
