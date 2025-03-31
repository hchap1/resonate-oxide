use iced::Element;
use iced::widget::text;
use iced::Task;

use crate::frontend::search_page::SearchPage;
use crate::frontend::message::Message;
use crate::frontend::message::PageType;

use crate::backend::filemanager::DataDir;
use crate::backend::filemanager::RefPackage;
use crate::backend::database::Database;
use crate::backend::util::{sync, AM};

pub trait Page<'a> {
    fn update(self: &mut Self, message: Message) -> Task<Message>;
    fn view(self: &'a Self) -> Element<'a, Message>;
}

pub struct Application<'a> {
    page: Option<Box<dyn Page<'a> + 'a>>,
    directories: DataDir,
    database: AM<Database>
}

impl<'a> Default for Application<'a> {
    fn default() -> Self {
        let datadir = match DataDir::create_or_load() {
            Ok(datadir) => datadir,
            Err(_) => panic!("Couldn't create suitable directory location")
        };

        let database = match Database::new(datadir.get_root_ref()) {
            Ok(database) => database,
            Err(_) => panic!("Couldn't create or load database")
        };

        Self::new(datadir, database)
    }
}

impl<'a> Application<'a> {
    pub fn new(directories: DataDir, database: Database) -> Self {
        let database = sync(database);
        Self {
            page: None,
            directories,
            database
        }
    }

    pub fn view(&'a self) -> Element<'a, Message> {
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
            PageType::SearchSongs => SearchPage::new(ref_package, self.database.clone())
        }));
    }
}
