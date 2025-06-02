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
    playlists: Vec<(Playlist, bool)>,
    editing: Option<usize>
}

impl PlaylistsPage {
    pub fn new(database: AM<Database>) -> Self {
        let playlists = {
            let unlocked_database = desync(&database);
            unlocked_database.retrieve_all_playlists()
        }.into_iter().map(|playlist| (playlist, false)).collect();

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
            column = column.push(
                ResonateWidget::hover_area(
                    ResonateWidget::playlist(
                        &value.0,
                        value.1,
                        match self.editing.map(|idx| (self.playlists[idx].0.name.as_str(), idx)) {
                            Some((playlist, idx)) => if idx == i { Some(playlist) } else { None },
                            None => None
                        }, i
                    ).on_press(Message::LoadPage(PageType::ViewPlaylist, Some(value.0.id))).into(),
                    i
                )
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
                    if !self.playlists.iter().any(|playlist| playlist.0.name == name) {
                        break (number, name);
                    }
                    number += 1;
                };

                let mut playlist: Playlist = Playlist {
                    id: 0,
                    name,
                };

                {
                    let database = desync(&self.database);
                    let id = match database.emplace_playlist_and_record_id(&playlist) {
                        Ok(id) => id,
                        Err(_) => return Task::none()
                    };
                    playlist.id = id;
                }

                self.playlists.push((playlist, false));

                Task::none()
            }

            Message::TextInput(text) => {
                match self.editing.as_ref() {
                    Some(playlist_idx) => {
                        match self.playlists.get_mut(*playlist_idx) {
                            Some(playlist) => playlist.0.name = text,
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
                    Some(idx) => database.set_playlist_name(self.playlists[idx].0.id, &self.playlists[idx].0.name),
                    None => {}
                };
                Task::none()
            }

            Message::DeletePlaylist(id) => {
                if let Some(idx) = self.playlists.iter().enumerate().find_map(|p|
                    if p.1.0.id == id { Some(p.0) }
                    else { None }
                ) {
                    self.playlists.remove(idx);
                }
                Task::none()
            }

            Message::Hover(idx, hover) => {
                if idx < self.playlists.len() {
                    self.playlists[idx].1 = hover;
                }
                Task::none()
            }

            _ => Task::none()
        }
    }

    fn back(&self, last_page: (PageType, Option<usize>)) -> (PageType, Option<usize>) {
        last_page
    }
}
