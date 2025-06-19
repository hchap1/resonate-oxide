use std::collections::HashSet;

use iced::alignment::Vertical;
use iced::widget::Column;
use iced::widget::Container;
use iced::widget::Row;
use iced::widget::text;
use iced::widget::Space;
use iced::Length;
use iced::Task;

use crate::backend::database_interface::DatabaseInterface;
use crate::backend::music::Playlist;
use crate::frontend::application::Page;
use crate::frontend::message::Message;
use crate::frontend::message::PageType;
use crate::frontend::widgets::ResonateWidget;
use crate::frontend::widgets::ResonateStyle;
use crate::frontend::widgets::ResonateColour;

use crate::backend::music::Song;
use crate::backend::filemanager::DataDir;
use crate::backend::database_manager::DataLink;

pub enum SpotifyNotification {
    Waiting(usize),
    Finished,
    NotAuthenticated,
    NoIdOrSecret,
    InvalidID,
    Success
}

pub struct ImportPage {
    database: DataLink,
    directories: DataDir,
    songs: Vec<Song>,
    input: String,
    notification: Option<SpotifyNotification>,

    spotify_id: Option<String>,
    spotify_client: Option<String>,
    
    playlist_name: Option<String>,
    playlist_size: Option<usize>,

    saved: bool,

    failed_again: bool
}

impl ImportPage {
    pub fn new(
        directories: DataDir,
        database: DataLink,
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
            playlist_name: None,
            playlist_size: None,
            saved: false,
            failed_again: false
        }
    }
}

impl Page for ImportPage {
    fn view(&self, current_song_downloads: &HashSet<String>, download_queue: &HashSet<Song>) -> Column<'_, Message> {

        let mut column = Column::new().spacing(20);

        for song in &self.songs {

            let is_downloading = current_song_downloads.contains(&song.yt_id);
            let is_queued = download_queue.contains(&song);

            let widget = ResonateWidget::song(
                song,
                self.directories.get_default_thumbnail(),
                is_downloading,
                is_queued,
                None,
                false
            );
            column = column.push(
                widget
            );
        }

        let notification_widget =
            self.notification.as_ref().map(
                |n| Container::new(match n {
                    SpotifyNotification::NoIdOrSecret => {
                        Row::new().padding(10).align_y(Vertical::Center)
                            .push(
                                text("No ID/SECRET").size(25).color(ResonateColour::red())
                                    .width(Length::Fill)
                            ).push(
                                Row::new().spacing(20).width(Length::Fill)
                                    .push(
                                        Space::new(Length::Fill, Length::Fixed(32f32))
                                    )
                                    .push(
                                        ResonateWidget::button_widget(crate::frontend::assets::close())
                                            .on_press(Message::ClearNotification)
                                    )
                            )
                    }
                    SpotifyNotification::NotAuthenticated => {
                        Row::new().padding(10).align_y(Vertical::Center)
                            .push(
                                text("Not Authenticated").size(25).color(
                                    match self.failed_again {
                                        true => ResonateColour::red(),
                                        false => ResonateColour::yellow()
                                    }
                                )
                                    .width(Length::Fill)
                            ).push(
                                Row::new().spacing(20).width(Length::Fill)
                                    .push(Space::new(Length::Fill, Length::Fixed(32f32)))
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
                    }
                    SpotifyNotification::InvalidID => {
                        Row::new().padding(10).align_y(Vertical::Center)
                            .push(
                                text("Not Authenticated").size(25).color(ResonateColour::red())
                            ).push(
                                Row::new().spacing(20).width(Length::Fill)
                                    .push(Space::new(Length::Fill, Length::Fixed(32f32)))
                                    .push(
                                        ResonateWidget::button_widget(crate::frontend::assets::close())
                                            .on_press(Message::ClearNotification)
                                    )
                            )
                    }
                    SpotifyNotification::Finished => {
                        Row::new().padding(10).align_y(Vertical::Center)
                            .push(
                                text("Finished").size(25).color(ResonateColour::green())
                            ).push(
                                Row::new().spacing(20).width(Length::Fill)
                                    .push(Space::new(Length::Fill, Length::Fixed(32f32)))
                                    .push(
                                        ResonateWidget::button_widget(crate::frontend::assets::close())
                                            .on_press(Message::ClearNotification)
                                    )
                            )
                    }
                    SpotifyNotification::Success => {
                        Row::new().padding(10).align_y(Vertical::Center)
                            .push(
                                text("Successfully Authenticated").size(25).color(ResonateColour::green())
                            ).push(
                                Row::new().spacing(20).width(Length::Fill)
                                    .push(Space::new(Length::Fill, Length::Fixed(32f32)))
                                    .push(
                                        ResonateWidget::button_widget(crate::frontend::assets::close())
                                            .on_press(Message::ClearNotification)
                                    )
                            )
                    }
                    SpotifyNotification::Waiting(received) => {
                        Row::new().padding(10).align_y(Vertical::Center)
                            .push(
                                text(
                                    format!("Not ready: {} / {} received.", received,
                                    self.playlist_size.unwrap_or(0))
                                ).size(25).color(ResonateColour::yellow())
                            ).push(
                                Row::new().spacing(20).width(Length::Fill)
                                    .push(Space::new(Length::Fill, Length::Fixed(32f32)))
                                    .push(
                                        ResonateWidget::button_widget(crate::frontend::assets::close())
                                            .on_press(Message::ClearNotification)
                                    )
                            )
                    }
                }).style(|_| ResonateStyle::list_container()).width(Length::Fill).align_y(Vertical::Center)
            );

        Column::new().spacing(20)
            .push(ResonateWidget::header("Spotify Playlist Import"))
            .push_maybe(
                match self.playlist_name.as_ref() {
                    Some(name) => Some(ResonateWidget::header(&name)),
                    None => None
                }
            )
            .push_maybe(notification_widget)
            .push(ResonateWidget::padded_scrollable(column.into()))
            .push(
                Row::new().spacing(20).align_y(Vertical::Center)
                    .push(
                        ResonateWidget::search_bar("Enter share link...", &self.input)
                            .on_paste(Message::TextInput)
                            .on_input(Message::TextInput)
                            .on_submit(Message::SpotifyPlaylist(self.input.clone()))
                    ).push_maybe(
                        if self.saved {
                            None
                        } else if self.playlist_size.is_some() {
                            if self.playlist_size.unwrap() == self.songs.len() {
                                Some(ResonateWidget::button_widget(crate::frontend::assets::save_icon())
                                    .on_press(Message::SavePlaylist)
                                )
                            } else {
                                None
                            }
                        } else {
                            None
                        }
                    )
            )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::SearchResult(song, _) => {
                self.songs.push(song);
                self.notification = Some(SpotifyNotification::Waiting(self.songs.len()));
                
                if let Some(size) = self.playlist_size {
                    if size == self.songs.len() {
                        self.notification = Some(SpotifyNotification::Finished)
                    }
                }
            },

            Message::SpotifyPlaylist(_) => {
                self.input.clear();
                self.songs.clear();
                self.saved = false;
            }

            Message::SpotifyAuthenticationFailedAgain => {
                self.notification = Some(SpotifyNotification::NotAuthenticated);
                self.failed_again = true;
            }

            Message::SpotifyAuthenticationSuccess => {
                self.notification = Some(SpotifyNotification::Success);
                self.failed_again = false;
            }

            Message::SpotifyAuthFailed => {
                if self.spotify_id.is_some() && self.spotify_client.is_some() {
                    self.notification = Some(SpotifyNotification::NotAuthenticated)
                } else {
                    self.notification = Some(SpotifyNotification::NoIdOrSecret)
                }
            }

            Message::SpotifyCreds(_, _) => {
                return Task::done(Message::ClearNotification);
            }

            Message::ClearNotification => {
                self.notification = None;
            }

            Message::TextInput(new_val) => {
                self.input = new_val
            }

            Message::SpotifyPlaylistName(name, size) => {
                self.playlist_name = Some(name);
                self.playlist_size = Some(size);
            }

            Message::SpotifyInvalidID => {
                self.notification = Some(SpotifyNotification::InvalidID);
            }

            Message::SavePlaylist => {

                self.saved = true;

                let playlist_name = match self.playlist_name.as_ref() {
                    Some(name) => name.clone(),
                    None => return Task::none()
                };

                let playlist = Playlist {
                    id: 0,
                    name: playlist_name
                };

                return Task::future(DatabaseInterface::insert_playlist(self.database.clone(), playlist))
                    .map(|playlist| Message::PlaylistCreated(playlist));
            }

            Message::PlaylistCreated(playlist) => {
                for song in self.songs.iter() {
                    DatabaseInterface::insert_playlist_entry(self.database.clone(), song.id, playlist.id)
                }
            }

            Message::UpdateThumbnails => {
                self.songs.iter_mut()
                    .for_each(|song|
                        song.load_paths(self.directories.get_music_ref(), self.directories.get_thumbnails_ref())
                    );
            }

            _ => {}
        }
        Task::none()
    }

    fn back(&self, previous: (PageType, Option<usize>)) -> (PageType, Option<usize>) {
        previous
    }
}
