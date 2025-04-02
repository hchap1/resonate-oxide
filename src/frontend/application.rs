use iced::Element;
use iced::widget::text;
use iced::Task;

use crate::frontend::search_page::SearchPage;
use crate::frontend::message::Message;
use crate::frontend::message::PageType;

use crate::backend::filemanager::DataDir;
use crate::backend::database::Database;
use crate::backend::util::{sync, AM};

pub trait Page {
    fn update(&mut self, message: Message) -> Task<Message>;
    fn view(&self) -> Element<'_, Message>;
}

pub struct Application<'a> {
    page: Option<Box<dyn Page + 'a>>,
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

        for song in database.retrieve_all_songs(datadir.get_music_ref(), datadir.get_thumbnails_ref()) {
            println!("Song: {song}");
        }

        let unique = database.is_unique(&String::from("9qnqYL0eNNI"));
        println!("{unique}");

        Self::new(datadir, database)
    }
}

impl Application<'_> {
    pub fn new(directories: DataDir, database: Database) -> Self {
        let database = sync(database);
        Self {
            page: Some(Box::new(SearchPage::new(directories.clone(), database.clone()))),
            directories,
            database
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
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

    fn load_page(&mut self, page_type: PageType) {
        self.page = Some(Box::new(match page_type {
            PageType::SearchSongs => SearchPage::new(self.directories.clone(), self.database.clone())
        }));
    }
}
