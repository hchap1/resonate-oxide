use iced::widget::Column;
use iced::Element;
use iced::widget::Row;
use iced::Length;
use iced::Task;

use crate::frontend::application::Page;
use crate::frontend::message::Message;
use crate::frontend::widgets::ResonateWidget;

use crate::backend::filemanager::DataDir;
use crate::backend::music::Playlist;
use crate::backend::music::Song;
use crate::backend::database::Database;
use crate::backend::util::AM;
use crate::backend::util::desync;

pub struct PlaylistPage {
    playlist: Playlist,
    songs: Vec<Song>,
    query: String,
    database: AM<Database>,
    directories: DataDir
}

impl PlaylistPage {
    pub fn new(playlist: Option<usize>, database: AM<Database>, directories: DataDir) -> Result<PlaylistPage, ()> {

        if playlist.is_none() {
            return Err(());
        }

        let (playlist, songs) = match {
            let database = desync(&database);
            (
                database.get_playlist_by_id(playlist.unwrap()),
                database.search_playlist(
                    playlist.unwrap(),
                    String::new(),
                    directories.get_music_ref(),
                    directories.get_thumbnails_ref()
                )
            )
        } {
            (Some(playlist), Ok(songs)) => (playlist, songs),
            _ => return Err(())
        };

        Ok(PlaylistPage {
            playlist,
            songs,
            query: String::new(),
            database,
            directories
        })
    }
}

impl Page for PlaylistPage {
    fn view(&self) -> Element<'_, Message> {
        let search_bar = Row::new().push(
            ResonateWidget::search_bar("Search...", &self.query)
        );

        let mut column = Column::new().spacing(20);

        for song in &self.songs {
            column = column.push(
                ResonateWidget::song(song, self.directories.get_default_thumbnail())
            );
        }

        let view_window = ResonateWidget::padded_scrollable(column.into()).width(Length::Fill).height(Length::Fill);
        
        ResonateWidget::window(
            Column::new()
                .push(Row::new()
                    .push(ResonateWidget::header(&self.playlist.name)))
                .push(view_window)
                .push(search_bar)
                .into()
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        Task::none()
    }
}
