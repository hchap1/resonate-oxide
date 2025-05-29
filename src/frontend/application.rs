use std::collections::HashSet;

use rand::rng;
use rand::seq::SliceRandom;

use iced::Element;
use iced::widget::Column;
use iced::widget::Row;
use iced::widget::text;
use iced::Length;
use iced::Task;

use crate::backend::audio::AudioTask;
use crate::backend::audio::QueueFramework;
use crate::backend::util::Relay;

// GUI PAGES
use crate::frontend::search_page::SearchPage;
use crate::frontend::playlists::PlaylistsPage;
use crate::frontend::playlist_page::PlaylistPage;

use crate::frontend::message::Message;
use crate::frontend::message::PageType;
use crate::frontend::backend_interface::async_download_thumbnail;
use crate::frontend::backend_interface::async_download_song;
use crate::frontend::backend_interface::async_install_dlp;
use crate::frontend::widgets::ResonateWidget;

use crate::backend::filemanager::DataDir;
use crate::backend::database::Database;
use crate::backend::util::{sync, AM};
use crate::backend::audio::AudioPlayer;

pub trait Page {
    fn update(&mut self, message: Message) -> Task<Message>;
    fn view(&self, current_song_downloads: &HashSet<String>) -> Column<'_, Message>;
    fn back(&self, previous_page: (PageType, Option<usize>)) -> (PageType, Option<usize>);
}

pub struct Application<'a> {
    page: Option<Box<dyn Page + 'a>>,
    directories: DataDir,
    database: AM<Database>,

    current_thumbnail_downloads: HashSet<String>,
    current_song_downloads: HashSet<String>,

    audio_player: Option<AudioPlayer>,
    queue_state: Option<QueueFramework>,

    last_page: (PageType, Option<usize>),
    current_page: (PageType, Option<usize>),
}

impl<'a> Default for Application<'a> {
    fn default() -> Self {
        let datadir = match DataDir::create_or_load() {
            Ok(datadir) => datadir,
            Err(_) => panic!("Couldn't create suitable directory location")
        };

        let database = match Database::new(datadir.get_root_ref()) {
            Ok(database) => database,
            Err(_) => panic!("Couldn't create or load database")
        };

        Self::new(datadir, database)
    }
}

impl Application<'_> {
    pub fn new(directories: DataDir, database: Database) -> Self {

        let database = sync(database);

        Self {
            page: Some(Box::new(PlaylistsPage::new(database.clone()))),
            directories,
            database,
            
            current_thumbnail_downloads: HashSet::new(),
            current_song_downloads: HashSet::new(),

            audio_player: None,
            queue_state: None,

            last_page: (PageType::Playlists, None),
            current_page: (PageType::Playlists, None)
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        match self.page.as_ref() {
            Some(page) => {
                ResonateWidget::window(
                    Column::new().spacing(20).push(
                        Row::new().spacing(20).push(
                            Column::new().spacing(20)
                                .push(page.view(&self.current_song_downloads))
                                .width(Length::FillPortion(3))
                            ).push(
                                Column::new().spacing(20).push(ResonateWidget::header("Queue"))
                                    .push(
                                        ResonateWidget::queue_bar(
                                            self.queue_state.as_ref(),
                                            self.directories.get_default_thumbnail()
                                        )
                                    )
                            )
                    ).push(ResonateWidget::control_bar(self.queue_state.as_ref(), 
                        match self.page.as_ref() {
                            Some(page) => page.back(self.last_page.clone()),
                            None => self.last_page.clone()
                        }
                    ))
                    .into()
                )
            }
            None => text("404 - No page.").into()
        }
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::DownloadDLP => {
                println!("[UPDATE] Check if DLP is downloaded");
                match self.directories.get_dlp_ref() {
                    Some(_) => { println!("ALREADY DOWNLOADED!"); Task::none() }
                    None => Task::future(async_install_dlp(self.directories.get_dependencies_ref().to_path_buf()))
                }
            }

            Message::DLPDownloaded(dlp_path) => {
                match dlp_path {
                    Some(dlp_path) => self.directories.take_dlp_path(dlp_path),
                    None => {}
                }

                Task::none()
            }

            Message::Download(song) => {

                if let Some(mp) = song.music_path.as_ref() {
                    if mp.exists() {
                        return Task::none();
                    }
                }

                if self.current_song_downloads.contains(&song.yt_id) {
                    return Task::none();
                }
                self.current_song_downloads.insert(song.yt_id.clone());
                Task::future(async_download_song(
                    self.directories.get_dlp_ref().map(|x| x.to_path_buf()),
                    self.directories.get_music_ref().to_path_buf(),
                    song
                ))
            }

            Message::SongDownloaded(song) => {
                self.current_song_downloads.remove(&song.yt_id);
                if let Some(page) = self.page.as_mut() {
                    let _ = page.update(Message::SongDownloaded(song));
                }
                Task::none()
            }

            Message::SearchResult(song) => {
                let id = song.yt_id.clone();
                let album = song.album.clone();

                let search_string = match song.album.as_ref() {
                    Some(album) => album.clone(),
                    None => id.clone()
                };

                let require_thumbnail_download = song.thumbnail_path.is_none() && !self.current_thumbnail_downloads.contains(&search_string);

                if let Some(page) = self.page.as_mut() {
                    let _ = page.update(Message::SearchResult(song));
                }

                if require_thumbnail_download {
                    self.current_thumbnail_downloads.insert(search_string);
                    Task::future(async_download_thumbnail(
                        self.directories.get_dlp_ref().expect("DLP not installed").to_path_buf(),
                        self.directories.get_thumbnails_ref().to_path_buf(),
                        id,
                        album
                    ))
                } else { Task::none() }
            }

            Message::MultiSearchResult(songs) => {
                Task::batch(songs.into_iter().map(|song| Task::done(Message::SearchResult(song))))
            }

            Message::LoadPage(page_type, playlist_id) => { self.load_page(page_type, playlist_id); Task::none() },

            Message::AddSongToPlaylist(song, playlist_id) => {
                let _ = self.database.lock().unwrap().add_song_to_playlist(song.id, playlist_id);
                Task::done(
                    Message::SongAddedToPlaylist(song.id)
                ).chain(Task::done(
                    Message::Download(song)
                ))
            }

            Message::AudioTask(task) => {
                if let Some(ap) = self.audio_player.as_ref() { let _ = ap.send_task(task); }
                Task::none()
            }

            Message::QueueUpdate(queue_state) => {
                self.queue_state = Some(queue_state);
                Task::none()
            }
            
            Message::LoadAudio => {
                let (audio_player, receiver) = match AudioPlayer::new() {
                    Ok(data) => data,
                    Err(_) => return Task::none()
                };

                self.audio_player = Some(audio_player);
                Task::stream(Relay::consume_receiver(receiver, |message| Message::QueueUpdate(message)))
            }

            Message::LoadEntirePlaylist(playlist_id, do_shuffle) => {
                let mut rng = rng();

                let mut songs = {
                    match self.database.lock().unwrap().search_playlist(
                        playlist_id, String::new(), 
                        self.directories.get_music_ref(),
                        self.directories.get_thumbnails_ref()
                    ) {
                        Ok(songs) => songs,
                        Err(_) => return Task::none()
                    }
                };

                if do_shuffle { songs.shuffle(&mut rng); }

                Task::done(Message::AudioTask(AudioTask::SetQueue(songs)))
            }

            Message::RemoveSongFromPlaylist(song_id, playlist_id) => {
                self.database.lock().unwrap().remove_song_from_playlist(song_id, playlist_id);
                if let Some(page) = self.page.as_mut() {
                    let _ = page.update(Message::RemoveSongFromPlaylist(song_id, playlist_id));
                }
                Task::done(Message::AudioTask(AudioTask::RemoveSongById(song_id)))
            }

            Message::DeletePlaylist(playlist_id) => {
                self.database.lock().unwrap().delete_playlist(playlist_id);
                if let Some(page) = self.page.as_mut() {
                    let _ = page.update(Message::DeletePlaylist(playlist_id));
                }
                Task::none()
            }

            Message::DownloadFailed(song) => {
                println!("[UPDATE] Download of {} failed.", song.title);
                self.current_song_downloads.remove(&song.yt_id);
                if let Some(page) = self.page.as_mut() {
                    let _ = page.update(Message::DownloadFailed(song));
                }
                Task::none()
            }

            other => match self.page.as_mut() {
                Some(page) => page.update(other),
                None => Task::none()
            }
        }
    }

    fn load_page(&mut self, page_type: PageType, playlist_id: Option<usize>) {
        self.last_page = self.current_page.to_owned();
        self.current_page = (page_type.clone(), playlist_id.clone());
        self.page = Some(match page_type {
            PageType::SearchSongs => Box::new(
                SearchPage::new(self.directories.clone(), self.database.clone(), playlist_id.unwrap())),
            PageType::Playlists => Box::new(PlaylistsPage::new(self.database.clone())),
            PageType::ViewPlaylist => Box::new(
                match PlaylistPage::new(playlist_id, self.database.clone(), self.directories.clone()) {
                    Ok(page) => page,
                    Err(_) => return // THIS SHOULD BE AN ERROR NOTIFICATION
                }
            )
        });
    }
}
