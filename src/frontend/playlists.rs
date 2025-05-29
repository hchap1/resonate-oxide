use std::collections::HashSet;

use iced::widget::Column;
use iced::Task;

use crate::frontend::application::Page;
use crate::frontend::message::Message;
use crate::frontend::widgets::ResonateWidget;
use crate::frontend::message::PageType;

use crate::backend::music::Playlist;
use crate::backend::database::Database;
use crate::backend::util::{AM, desync};

pub struct PlaylistsPage {
    database: AM<Database>,
    playlists: Vec<Playlist>,
    editing: Option<usize>
}

impl PlaylistsPage {
    pub fn new(database: AM<Database>) -> Self {
        let playlists = {
            let unlocked_database = desync(&database);
            unlocked_database.retrieve_all_playlists()
        };

        Self {
            database,
            playlists,
            editing: None
        }
    }
}

impl Page for PlaylistsPage {
    fn view(&self, _current_song_downloads: &HashSet<String>) -> Column<'_, Message> {
        let mut column = Column::new().spacing(20);
        for (i, value) in self.playlists.iter().enumerate() {
            column = column.push(ResonateWidget::playlist(value,
            match self.editing.map(|idx| (self.playlists[idx].name.as_str(), idx)) {
                Some((playlist, idx)) => if idx == i { Some(playlist) } else { None },
                None => None
            }, i)
                .on_press(Message::LoadPage(PageType::ViewPlaylist, Some(value.id)))
            );
        }

        let view_window = ResonateWidget::padded_scrollable(
            column
                .push(
                    ResonateWidget::inline_button("+ Create Playlist")
                        .on_press(Message::CreatePlaylist)
                )
                .into()
        );

            Column::new().spacing(20)
                .push(ResonateWidget::header("Playlists"))
                .push(view_window)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::CreatePlaylist => {
                let mut number: usize = 1;
                let (_, name) = loop {
                    let name = format!("Playlist #{number}");
                    if !self.playlists.iter().any(|playlist| playlist.name == name) {
                        break (number, name);
                    }
                    number += 1;
                };

                let mut playlist: Playlist = Playlist {
                    id: 0,
                    name,
                    song_count: 0
                };

                {
                    let database = desync(&self.database);
                    let id = match database.emplace_playlist_and_record_id(&playlist) {
                        Ok(id) => id,
                        Err(_) => return Task::none()
                    };
                    playlist.id = id;
                }

                self.playlists.push(playlist);

                Task::none()
            }

            Message::TextInput(text) => {
                match self.editing.as_ref() {
                    Some(playlist_idx) => {
                        match self.playlists.get_mut(*playlist_idx) {
                            Some(playlist) => playlist.name = text,
                            None => {}
                        }
                    }
                    None => {}
                }
                Task::none()
            }

            Message::StartEditing(idx) => {
                self.editing = Some(idx);
                Task::none()
            }

            Message::StopEditing => {
                let database = desync(&self.database);
                match self.editing.take() {
                    Some(idx) => database.set_playlist_name(self.playlists[idx].id, &self.playlists[idx].name),
                    None => {}
                };
                Task::none()
            }

            _ => Task::none()
        }
    }

    fn back(&self, last_page: (PageType, Option<usize>)) -> (PageType, Option<usize>) {
        last_page
    }
}
