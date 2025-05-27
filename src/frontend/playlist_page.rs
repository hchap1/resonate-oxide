use iced::Element;
use iced::widget::Row;
use iced::Task;

use crate::frontend::application::Page;
use crate::frontend::message::Message;
use crate::frontend::widgets::ResonateWidget;

use crate::backend::music::Playlist;
use crate::backend::music::Song;
use crate::backend::database::Database;
use crate::backend::util::AM;
use crate::backend::util::desync;

pub struct PlaylistPage {
    playlist: Playlist,
    songs: Vec<Song>,
    query: String,
    database: AM<Database>
}

impl PlaylistPage {
    pub fn new(playlist: Playlist, database: AM<Database>) -> PlaylistPage {

        {
            let database = desync(&database);
            database.
        }

        PlaylistPage {
            playlist,
            songs: 
            query: String::new(),
            database
        }
    }
}

impl Page for PlaylistPage {
    fn view(&self) -> Element<'_, Message> {
        let search_bar = Row::new().push(
            ResonateWidget::search_bar("Search...", &self.query)
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        ResonateWidget::song()
    }
}
