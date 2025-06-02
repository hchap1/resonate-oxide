use std::collections::HashSet;

use iced::widget::Column;
use iced::widget::Container;
use iced::widget::Row;
use iced::widget::text;
use iced::Length;
use iced::Task;

use crate::backend::music::Playlist;
use crate::frontend::application::Page;
use crate::frontend::message::Message;
use crate::frontend::message::PageType;
use crate::frontend::widgets::ResonateWidget;

use crate::backend::music::Song;
use crate::backend::filemanager::DataDir;
use crate::backend::database::Database;
use crate::backend::util::AM;
use crate::backend::util::desync;

use super::widgets::ResonateColour;

pub enum SpotifyNotification {
    NotAuthenticated,
    NoIdOrSecret
}

pub struct ImportPage {
    database: AM<Database>,
    directories: DataDir,
    songs: Vec<Song>,
    input: String,
    notification: Option<SpotifyNotification>,

    spotify_id: Option<String>,
    spotify_client: Option<String>,
    
    playlist_name: Option<String>
}

impl ImportPage {
    pub fn new(
        directories: DataDir,
        database: AM<Database>,
        spotify_id: Option<String>,
        spotify_client: Option<String>,
    ) -> ImportPage {
        ImportPage {
            database,
            directories,
            songs: Vec::new(),
            input: String::new(),
            notification: None,
            spotify_id,
            spotify_client,
            playlist_name: None
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

        let column = column.push_maybe(
            self.notification.as_ref().map(
                |n| match n {
                    SpotifyNotification::NoIdOrSecret => {
                        Container::new(
                            Row::new()
                                .push(
                                    text("No ID/SECRET").size(25).color(ResonateColour::red())
                                        .width(Length::Fill)
                                ).push(
                                    Row::new().spacing(20).width(Length::Fill)
                                        .push(
                                            ResonateWidget::button_widget(crate::frontend::assets::close())
                                                .on_press(Message::ClearNotification)
                                        )
                                )
                        )
                    }
                    SpotifyNotification::NotAuthenticated => {
                        Container::new(
                            Row::new()
                                .push(
                                    text("Not Authenticated").size(25).color(ResonateColour::yellow())
                                        .width(Length::Fill)
                                ).push(
                                    Row::new().spacing(20).width(Length::Fill)
                                        .push(
                                            ResonateWidget::button_widget(crate::frontend::assets::refresh())
                                                .on_press(Message::SpotifyCreds(
                                                    self.spotify_id.clone(),
                                                    self.spotify_client.clone()
                                                )
                                            )
                                        ).push(
                                            ResonateWidget::button_widget(crate::frontend::assets::close())
                                                .on_press(Message::ClearNotification)
                                        )
                                    )
                                )
                    }
                }
            )
        );

        Column::new().spacing(20)
            .push(ResonateWidget::header("Spotify Playlist Import"))
            .push_maybe(
                match self.playlist_name.as_ref() {
                    Some(name) => Some(ResonateWidget::header(&name)),
                    None => None
                }
            )
            .push(ResonateWidget::padded_scrollable(column.into()))
            .push(
                Row::new()
                    .push(
                        ResonateWidget::search_bar("Enter share link...", &self.input)
                            .on_paste(Message::TextInput)
                            .on_input(Message::TextInput)
                            .on_submit(Message::SpotifyPlaylist(self.input.clone()))
                            .width(Length::Fill)
                    ).push(
                        ResonateWidget::button_widget(crate::frontend::assets::save_icon())
                            .on_press(Message::SavePlaylist)
                    )
            )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SearchResult(song, _) => {
                self.songs.push(song)
            },

            Message::SpotifyAuthFailed => {
                self.notification = Some(SpotifyNotification::NotAuthenticated)
            }

            Message::ClearNotification => {
                self.notification = None;
            }

            Message::TextInput(new_val) => self.input = new_val,
            Message::SpotifyPlaylistName(name) => {
                self.playlist_name = Some(name);
            }

            Message::SavePlaylist => {
                let playlist_name = match self.playlist_name.as_ref() {
                    Some(name) => name.clone(),
                    None => return Task::none()
                };

                let mut playlist = Playlist {
                    id: 0,
                    name: playlist_name
                };

                let id = match desync(&self.database).emplace_playlist_and_record_id(&playlist) {
                    Ok(id) => id,
                    Err(_) => return Task::none()
                };

                playlist.id = id;

                for song in self.songs.iter() {
                    let _ = desync(&self.database).add_song_to_playlist(song.id, playlist.id);
                }
            }

            _ => {}
        }
        Task::none()
    }

    fn back(&self, previous: (PageType, Option<usize>)) -> (PageType, Option<usize>) {
        previous
    }
}
