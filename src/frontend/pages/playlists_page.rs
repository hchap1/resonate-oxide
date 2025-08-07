use std::collections::HashSet;

use iced::widget::Column;
use iced::Task;

use crate::backend::database_interface::DatabaseInterface;
use crate::backend::thumbnail::ThumbnailManager;
use crate::frontend::application::Page;
use crate::frontend::message::Message;
use crate::frontend::widgets::ResonateWidget;
use crate::frontend::message::PageType;

use crate::backend::music::{Playlist, Song};
use crate::backend::database_manager::DataLink;

pub struct PlaylistsPage {
    database: DataLink,
    playlists: Vec<(Playlist, bool)>,
    editing: Option<usize>
}

impl PlaylistsPage {
    pub fn new(database: DataLink) -> Self {
        Self {
            database,
            playlists: Vec::new(),
            editing: None
        }
    }
}

impl Page for PlaylistsPage {
    fn view(
        &self, _current_song_downloads: &HashSet<String>, _queued_downloads: &HashSet<Song>, _: &ThumbnailManager
    ) -> Column<'_, Message> {
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
                ).push(
                    ResonateWidget::inline_button("+ Import Spotify Playlist")
                        .on_press(Message::LoadPage(PageType::ImportSpotify, None))
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

                let playlist: Playlist = Playlist {
                    id: 0,
                    name,
                };

                Task::future(DatabaseInterface::insert_playlist(self.database.clone(), playlist))
                    .map(Message::PlaylistCreated)
            }

            Message::PlaylistCreated(playlist) => {
                self.playlists.push((playlist, false));
                Task::none()
            }

            Message::TextInput(text) => {
                if let Some(playlist_idx) = self.editing.as_ref() {
                    if let Some(playlist) = self.playlists.get_mut(*playlist_idx) { playlist.0.name = text }
                }
                Task::none()
            }

            Message::StartEditing(idx) => {
                self.editing = Some(idx);
                Task::none()
            }

            Message::StopEditing => {
                if let Some(idx) = self.editing.take() {
                    DatabaseInterface::update_playlist_name(
                        self.database.clone(),
                        self.playlists[idx].0.clone()
                    )
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

            Message::PlaylistLoaded(playlist) => {
                self.playlists.push((playlist, false));
                Task::none()
            }

            _ => Task::none()
        }
    }

    fn back(&self, last_page: (PageType, Option<usize>)) -> (PageType, Option<usize>) {
        last_page
    }
}
