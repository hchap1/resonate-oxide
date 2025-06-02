use std::collections::HashSet;

use iced::widget::Column;
use iced::Task;

use crate::frontend::application::Page;
use crate::frontend::message::Message;
use crate::frontend::message::PageType;
use crate::frontend::widgets::ResonateWidget;

use crate::backend::music::Song;
use crate::backend::filemanager::DataDir;
use crate::backend::database::Database;
use crate::backend::util::AM;

pub struct ImportPage {
    database: AM<Database>,
    directories: DataDir,
    songs: Vec<Song>
}

impl ImportPage {
    pub fn new(directories: DataDir, database: AM<Database>) -> ImportPage {
        ImportPage {
            database,
            directories,
            songs: Vec::new()
        }
    }
}

impl Page for ImportPage {
    fn view(&self, current_song_downloads: &HashSet<String>) -> Column<'_, Message> {

        let mut column = Column::new().spacing(20);

        for song in &self.songs {
            let is_downloading = current_song_downloads.contains(&song.yt_id);
            let widget = ResonateWidget::song(
                song,
                self.directories.get_default_thumbnail(),
                is_downloading,
                None,
                false
            );
            column = column.push(
                widget
            );
        }

        Column::new().spacing(20)
            .push(ResonateWidget::header("Import Spotify Playlist"))
            .push(ResonateWidget::padded_scrollable(column.into()))
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SearchResult(song, _) => {
                self.songs.push(song)
            },

            _ => {}
        }
        Task::none()
    }

    fn back(&self, previous: (PageType, Option<usize>)) -> (PageType, Option<usize>) {
        previous
    }
}
