use std::collections::HashSet;

use iced::alignment::Vertical;
use iced::widget::Column;
use iced::widget::Container;
use iced::widget::Row;
use iced::widget::text;
use iced::Length;
use iced::Task;

use crate::backend::database_interface::DatabaseInterface;
use crate::frontend::application::Page;
use crate::frontend::message::Message;
use crate::frontend::widgets::ResonateWidget;
use crate::frontend::widgets::ResonateStyle;
use crate::frontend::widgets::ResonateColour;
use crate::frontend::message::PageType;

use crate::backend::filemanager::DataDir;
use crate::backend::music::Playlist;
use crate::backend::music::Song;
use crate::backend::database_manager::DataLink;
use crate::backend::util::Relay;
use crate::backend::util::consume;

pub struct PlaylistPage {
    playlist: Playlist,
    songs: Vec<Song>,
    query: String,
    database: DataLink,
    directories: DataDir,
    hovered_song: Option<usize>,
    
    total_songs: usize,
    downloaded: usize
}

impl PlaylistPage {
    pub fn new(playlist: Option<usize>, database: DataLink, directories: DataDir) -> Result<PlaylistPage, ()> {

        if playlist.is_none() {
            return Err(());
        }

        // Need to asynchronously get playlist name, songs in playlist
        // Need to asynchronously get download info

        Ok(PlaylistPage {
            playlist: Playlist { name: String::from("Loading..."), id: playlist.unwrap() },
            songs: Vec::new(),
            query: String::new(),
            database,
            directories,
            hovered_song: None,
            total_songs: 0,
            downloaded: 0
        })
    }
}

impl Page for PlaylistPage {
    fn view(
        &self, current_song_downloads: &HashSet<String>, queued_downloads: &HashSet<Song>
    ) -> Column<'_, Message> {
        let search_bar = Row::new().spacing(20).align_y(Vertical::Center).push(
            ResonateWidget::search_bar("Search...", &self.query)
                .on_input(Message::TextInput)
                .on_submit(Message::SubmitSearch))
            .push(
                ResonateWidget::inline_button("ADD SONGS")
                    .on_press(Message::LoadPage(PageType::SearchSongs, Some(self.playlist.id)))
            );

        let mut column = Column::new().spacing(20);

        for song in &self.songs {

            let is_downloading = current_song_downloads.contains(&song.yt_id);
            let is_queued = queued_downloads.contains(&song);

            if is_downloading && is_queued {
                println!("[ALERT] Queue / Download collision.");
            }

            let widget = ResonateWidget::song(
                song,
                self.directories.get_default_thumbnail(),
                is_downloading,
                is_queued,
                Some(self.playlist.id),
                if let Some(id) = self.hovered_song { id == song.id } else { false }
            );
            column = column.push(
                ResonateWidget::hover_area(
                    if song.music_path.is_none() {
                        widget.on_press(Message::Download(song.clone()))
                    } else {
                        widget.on_press(Message::AudioTask(crate::backend::audio::AudioTask::Push(song.clone())))
                    }.into(),
                    song.id
                )
            );
        }

        let view_window = ResonateWidget::padded_scrollable(column.into()).width(Length::Fill).height(Length::Fill);
        
            Column::new().spacing(20)
                .push(Row::new()
                    .push(ResonateWidget::header(&self.playlist.name)))
                .push_maybe(
                    if self.downloaded < self.total_songs {Some(
                        Container::new(Row::new().spacing(20).padding(10).align_y(Vertical::Center)
                            .push(
                                text(
                                    format!("{} / {} downloaded", self.downloaded, self.total_songs)
                                ).color(ResonateColour::text()).size(32).width(Length::Fill)
                            ).push(
                                ResonateWidget::coloured_icon_button(
                                    crate::frontend::assets::downloading_icon(),
                                    ResonateColour::text()
                                ).on_press(Message::DownloadAll(
                                    self.songs
                                        .iter()
                                        .filter_map(
                                            |song| if song.music_path.is_some() { None } else { Some(song.clone()) }
                                        )
                                        .collect::<Vec<Song>>()
                                ))
                            )
                        ).style(|_| ResonateStyle::list_container()).width(Length::Fill)
                    )} else {
                        None
                    }
                )
                .push(view_window)
                .push(search_bar)
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {

            Message::SongStream(song) => {
                self.total_songs += 1;
                if song.music_path.is_some() {
                    self.downloaded += 1;
                }
                self.songs.push(song);
            }

            Message::TextInput(new_value) => self.query = new_value,

            Message::SubmitSearch => {
                let query = consume(&mut self.query);
                return Task::stream(Relay::consume_receiver(DatabaseInterface::select_all_songs_in_playlist(
                    self.database.clone(),
                    self.playlist.id
                ), move |item_stream| match item_stream {
                    crate::backend::database_manager::ItemStream::End => None,
                    crate::backend::database_manager::ItemStream::Error => None,
                    crate::backend::database_manager::ItemStream::Value(row) => {
                        Some(Message::RowIntoSongQuery(row, query.clone()))
                    }
                }));
            }

            Message::SongDownloaded(song) => {
                for s in &mut self.songs {
                    if s.id == song.id {
                        s.load_paths(
                            self.directories.get_music_ref(),
                            self.directories.get_thumbnails_ref()
                        );
                        self.downloaded += 1;
                    }
                }
            }

            Message::DownloadFailed(_) => {},

            Message::RemoveSongFromPlaylist(song_id, _) => {
                match self.songs.iter().enumerate().find_map(|song|
                    if song.1.id == song_id { Some(song.0) }
                    else { None }
                ) {
                    Some(idx) => { self.songs.remove(idx); },
                    None => {}
                }
            }

            Message::Hover(id, hover) => {
                if hover { self.hovered_song = Some(id) }
                else { self.hovered_song = None; }
            }

            Message::UpdateThumbnails => {
                self.songs.iter_mut()
                    .for_each(|song|
                        song.load_paths(self.directories.get_music_ref(), self.directories.get_thumbnails_ref())
                    );
            }

            _ => ()
        }.into()
    }

    fn back(&self, _: (PageType, Option<usize>)) -> (PageType, Option<usize>) {
        (PageType::Playlists, None)
    }
}
