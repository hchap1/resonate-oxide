use std::collections::HashSet;

use iced::alignment::Vertical;
use iced::futures::FutureExt;
use iced::widget::Column;
use iced::task::Handle;
use iced::widget::Row;
use iced::Length;
use iced::Task;

use crate::backend::database_interface::DatabaseInterface;
use crate::frontend::application::Page;
use crate::frontend::message::PageType;
use crate::frontend::widgets::ResonateWidget;

use crate::backend::util::consume;
use crate::backend::music::Playlist;
use crate::frontend::message::Message;
use crate::backend::filemanager::DataDir;
use crate::backend::database_manager::DataLink;
use crate::backend::music::Song;
use crate::backend::web::flatsearch;
use crate::backend::web::AsyncMetadataCollectionPool;
use crate::backend::util::Relay;

pub enum SearchState {
    Searching,
    SearchFailed,
    Received(Vec<Song>)
}

pub struct SearchPage {
    query: String,
    directories: DataDir,
    database: DataLink,
    search_results: Option<Vec<Song>>,
    search_handles: Vec<Handle>,
    playlist: Option<Playlist>,
    existing_songs: HashSet<usize>,
    search_notify: Option<SearchState>
}

impl SearchPage {
    pub fn new(directories: DataDir, database: DataLink, playlist_id: usize) -> Self {
        let playlist = Playlist { name: String::from("Loading..."), id: playlist_id };
        Self {
            query: String::new(),
            directories,
            database,
            search_results: None,
            search_handles: Vec::new(),
            playlist: Some(playlist),
            existing_songs: HashSet::new(),
            search_notify: None
        }
    }
}

impl Page for SearchPage {
    fn view(
        &self, current_song_downloads: &HashSet<String>, queued_downloads: &HashSet<Song>
    ) -> Column<'_, Message> {
        let search_bar = Row::new()
            .push(
                ResonateWidget::search_bar("Search...", &self.query)
                    .on_input(Message::TextInput)
                    .on_submit(Message::SubmitSearch)
            );

        let mut column = Column::new().spacing(20)
            .push_maybe(
                match self.search_notify.as_ref() {
                    Some(notify) => {
                        if let Some(playlist) = self.playlist.as_ref() {
                            Some(ResonateWidget::search_notify(
                                notify,
                                self.directories.get_default_thumbnail(),
                                playlist.id
                            ))
                        } else {
                            None
                        }
                    },
                    None => None
                }
            );


        if let Some(search_results) = self.search_results.as_ref() {
            for song in search_results {

                if self.existing_songs.contains(&song.id) {
                    continue;
                }

                let is_downloading = current_song_downloads.contains(&song.yt_id);
                let is_queued = queued_downloads.contains(&song);

                if is_downloading && is_queued {
                    println!("[ALERT] Queue / Download collision.");
                }

                column = column.push(
                    ResonateWidget::song(
                        song,
                        self.directories.get_default_thumbnail(),
                        is_downloading,
                        is_queued,
                        None,
                        false)
                        .on_press(Message::AddSongToPlaylist(song.clone(), match self.playlist.as_ref() {
                            Some(playlist) => playlist.id,
                            None => 0
                        })
                    )
                )
            }
        }

        let view_window = ResonateWidget::padded_scrollable(column.into());

        Column::new().spacing(20)
            .push(Row::new().push(ResonateWidget::header(
                match self.playlist.as_ref() {
                    Some(playlist) => &playlist.name,
                    None => "Search"
                }
            )).push(
                ResonateWidget::header(" - Add Songs")
            ).spacing(20).align_y(Vertical::Center).width(Length::Fill))
            .push(view_window)
            .push(search_bar)
    }

    fn update(self: &mut Self, message: Message) -> Task<Message> {
        match message {

            Message::SongStream(song) => {
                self.existing_songs.insert(song.id);

                if let Some(search_results) = self.search_results.as_mut() {
                    search_results.push(song)
                } else {
                    self.search_results = Some(vec![song])
                }
                Task::none()
            }

            Message::TextInput(new_value) => { self.query = new_value; Task::none() }

            Message::SubmitSearch => {

                self.search_notify = Some(SearchState::Searching);

                if let Some(search_results) = self.search_results.as_mut() { search_results.clear(); }
                for handle in &self.search_handles { handle.abort(); }

                let dlp_path = match self.directories.get_dlp_ref() {
                    Some(dlp_path) => dlp_path.to_path_buf(),
                    None => return Task::none()
                };

                let (flatsearch_task, flatsearch_handle) = Task::<Message>::future(
                    flatsearch(dlp_path, self.query.clone()).map(|res| match res {
                        Ok(results) => Message::LoadSearchResults(results),
                        Err(_) => Message::DLPWarning

                    })
                ).abortable();

                self.search_handles.push(flatsearch_handle);
                let query = consume(&mut self.query);

                Task::<Message>::stream(
                    Relay::consume_receiver(DatabaseInterface::select_all_songs(self.database.clone()),
                        move |item_stream| match item_stream {
                            crate::backend::database_manager::ItemStream::End => None,
                            crate::backend::database_manager::ItemStream::Error => None,
                            crate::backend::database_manager::ItemStream::Value(row) => {
                                Some(Message::RowIntoSearchResult(row, query.clone()))
                            }
                        }
                    )
                ).chain(
                    flatsearch_task
                )
            }

            Message::LoadSearchResults(search_results) => {
                let ids = match search_results.len() > 3 {
                    true => search_results[0..3].to_vec(),
                    false => search_results
                };
                let (metadata_collector, metadata_collection_handle) = Task::stream(
                    AsyncMetadataCollectionPool::new(
                        ids,
                        match self.directories.get_dlp_ref() {
                            Some(dlp_ref) => Some(dlp_ref.to_path_buf()),
                            None => None
                        },
                        self.directories.get_music_ref().to_path_buf(),
                        self.directories.get_thumbnails_ref().to_path_buf(),
                        self.database.clone()
                    )
                ).abortable();

                self.search_handles.push(metadata_collection_handle);
                metadata_collector.map(|song_batch| Message::MultiSearchResult(song_batch, true))
            }

            Message::SearchResult(song, from_online) => {
                if from_online {
                    if let Some(notify) = self.search_notify.as_mut() {
                        let mut current = match notify {
                            SearchState::Received(songs) => songs.clone(),
                            _ => vec![]
                        };

                        current.push(song);
                        *notify = SearchState::Received(current);
                        return Task::none();
                    }
                }

                match self.search_results.as_mut() {
                    Some(search_results) => search_results.push(song),
                    None => self.search_results = Some(vec![song])
                }

                Task::none()
            }
            
            Message::DLPWarning => {
                match self.search_notify.as_mut() {
                    Some(notify) => match notify {
                        SearchState::Received(songs) => {
                            match self.search_results.as_mut() {
                                Some(search_results) => search_results.append(songs),
                                None => self.search_results = Some(songs.to_vec())
                            }
                        },
                        _ => {}
                    }
                    None => {}
                }

                self.search_notify = Some(SearchState::SearchFailed);
                Task::none()
            }

            Message::UpdateThumbnails => {
                if let Some(search_results) = self.search_results.as_mut() {
                    search_results.iter_mut()
                        .for_each(|song|
                            song.load_paths(self.directories.get_music_ref(), self.directories.get_thumbnails_ref())
                        );
                }

                if let Some(notify) = self.search_notify.as_mut() {
                    if let SearchState::Received(songs) = notify {
                        songs.iter_mut()
                            .for_each(|song|
                                song.load_paths(
                                    self.directories.get_music_ref(),
                                    self.directories.get_thumbnails_ref())
                            );
                    }
                }

                Task::none()
            }

            Message::SongAddedToPlaylist(song_id) => {
                self.existing_songs.insert(song_id);
                Task::none()
            }

            Message::RemoveSearchStatus => {
                match self.search_notify.as_mut() {
                    Some(notify) => match notify {
                        SearchState::Received(songs) => {
                            match self.search_results.as_mut() {
                                Some(search_results) => search_results.append(songs),
                                None => self.search_results = Some(songs.to_vec())
                            }
                        },
                        _ => {}
                    }
                    None => {}
                }

                self.search_notify = None;
                Task::none()
            }

            _ => Task::none()
        }
    }

    fn back(&self, last_page: (PageType, Option<usize>)) -> (PageType, Option<usize>) {
        last_page
    }
}
