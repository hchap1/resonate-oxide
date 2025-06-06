use std::collections::HashSet;

use iced::alignment::Vertical;
use rand::rng;
use rand::seq::SliceRandom;

use iced::Element;
use iced::widget::Column;
use iced::widget::Row;
use iced::widget::text;
use iced::Length;
use iced::Task;

use rspotify::model::PlayableItem;

use crate::backend::audio::AudioTask;
use crate::backend::audio::ProgressUpdate;
use crate::backend::audio::QueueFramework;
use crate::backend::music::Song;
use crate::backend::settings::Settings;
use crate::backend::spotify::SpotifySongStream;
use crate::backend::util::Relay;
use crate::backend::util::desync;
use crate::backend::spotify::try_auth;
use crate::backend::spotify::load_spotify_song;
use crate::backend::spotify::SpotifyEmmision;

// GUI PAGES
use crate::frontend::search_page::SearchPage;
use crate::frontend::playlists::PlaylistsPage;
use crate::frontend::playlist_page::PlaylistPage;
use crate::frontend::import_page::ImportPage;

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
    fn view(&self, current_song_downloads: &HashSet<String>, queued_downloads: &HashSet<Song>) -> Column<'_, Message>;
    fn back(&self, previous_page: (PageType, Option<usize>)) -> (PageType, Option<usize>);
}

pub struct Application<'a> {
    settings: Settings,
    page: Option<Box<dyn Page + 'a>>,
    directories: DataDir,
    database: AM<Database>,

    current_thumbnail_downloads: HashSet<String>,
    current_song_downloads: HashSet<String>,
    download_queue: HashSet<Song>,

    audio_player: Option<AudioPlayer>,
    queue_state: Option<QueueFramework>,
    progress_state: Option<ProgressUpdate>,
    volume: f32,

    last_page: (PageType, Option<usize>),
    current_page: (PageType, Option<usize>),
    default_queue: QueueFramework,

    spotify_credentials: Option<rspotify::ClientCredsSpotify>,
    spotify_id: Option<String>,
    spotify_secret: Option<String>
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
            settings: Settings::default(),
            page: Some(Box::new(PlaylistsPage::new(database.clone()))),
            directories,
            database,
            
            current_thumbnail_downloads: HashSet::new(),
            current_song_downloads: HashSet::new(),
            download_queue: HashSet::new(),

            audio_player: None,
            queue_state: None,
            progress_state: None,
            volume: 1f32,

            last_page: (PageType::Playlists, None),
            current_page: (PageType::Playlists, None),
            default_queue: QueueFramework::default(),
            
            spotify_credentials: None,
            spotify_id: None,
            spotify_secret: None
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        match self.page.as_ref() {
            Some(page) => {
                ResonateWidget::window(
                    Column::new().spacing(20).push(
                        Row::new().spacing(20).push(
                            Column::new().spacing(20)
                                .push(page.view(&self.current_song_downloads, &self.download_queue))
                                .width(Length::FillPortion(3))
                            ).push(
                                Column::new().spacing(20)
                                    .push(
                                        Row::new().align_y(Vertical::Center)
                                            .push(ResonateWidget::header("Queue"))
                                            .push(
                                                ResonateWidget::inline_button("Clear")
                                                    .on_press(Message::AudioTask(AudioTask::ClearQueue))
                                            )
                                    )
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
                        },
                        self.progress_state.clone(),
                        self.volume,
                        self.directories.get_default_thumbnail(),
                        &self.default_queue
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

                let is_space: bool = self.current_song_downloads.len() > self.settings.max_download_concurrency;

                if self.download_queue.contains(&song) && !is_space {
                    return Task::none();
                }

                if is_space {
                    self.download_queue.insert(song.clone());
                    Task::none()
                } else {
                    let _ = self.download_queue.remove(&song);
                    self.current_song_downloads.insert(song.yt_id.clone());
                    Task::future(async_download_song(
                        self.directories.get_dlp_ref().map(|x| x.to_path_buf()),
                        self.directories.get_music_ref().to_path_buf(),
                        song
                    ))
                }
            }

            Message::SongDownloaded(song) => {
                self.current_song_downloads.remove(&song.yt_id);
                if let Some(page) = self.page.as_mut() {
                    let _ = page.update(Message::SongDownloaded(song));
                }

                if self.download_queue.len() > 0 {
                    let song = match self.download_queue.iter().nth(0) {
                        Some(song) => song.clone(),
                        None => return Task::none()
                    };
                    let _ = self.download_queue.remove(&song);
                    Task::done(Message::Download(song))
                } else {
                    Task::none()
                }
            }

            Message::SearchResult(song, from_online) => {
                let id = song.yt_id.clone();
                let album = song.album.clone();

                let search_string = match song.album.as_ref() {
                    Some(album) => album.clone(),
                    None => id.clone()
                };

                let require_thumbnail_download = song.thumbnail_path.is_none()
                    && !self.current_thumbnail_downloads.contains(&search_string);

                if let Some(page) = self.page.as_mut() {
                    let _ = page.update(Message::SearchResult(song, from_online));
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

            Message::MultiSearchResult(songs, is_online) => {
                Task::batch(songs.into_iter().map(|song| Task::done(Message::SearchResult(song, is_online))))
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
                if let AudioTask::SetVolume(v) = task { self.volume = v; }
                if let Some(ap) = self.audio_player.as_ref() { let _ = ap.send_task(task); }
                Task::none()
            }

            Message::QueueUpdate(queue_state) => {
                self.queue_state = Some(queue_state);
                Task::none()
            }
            
            Message::LoadAudio => {
                let (audio_player, queue_receiver, progress_receiver) = match AudioPlayer::new() {
                    Ok(data) => data,
                    Err(_) => return Task::none()
                };

                self.audio_player = Some(audio_player);
                Task::batch(vec![
                    Task::stream(
                        Relay::consume_receiver(queue_receiver, |message| Message::QueueUpdate(message))
                    ),
                    Task::stream(
                        Relay::consume_receiver(progress_receiver, |message| Message::ProgressUpdate(message))
                    )
                ])
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

                if self.download_queue.len() > 0 {
                    let song = match self.download_queue.iter().nth(0) {
                        Some(song) => song.clone(),
                        None => return Task::none()
                    };
                    let _ = self.download_queue.remove(&song);
                    Task::done(Message::Download(song))
                } else {
                    Task::none()
                }
            }

            Message::ProgressUpdate(update) => {
                self.progress_state = Some(update);
                Task::none()
            }

            Message::SpotifyCreds(id, secret) => {

                if let Some(page) = self.page.as_mut() {
                    let _ = page.update(Message::SpotifyCreds(id.clone(), secret.clone()));
                }

                if let (Some(id), Some(secret)) = (id, secret) {
                    self.spotify_id = Some(id.clone());
                    self.spotify_secret = Some(secret.clone());
                    Task::future(
                        try_auth(
                            rspotify::ClientCredsSpotify::new(
                                rspotify::Credentials::new(
                                    &id, &secret
                                )
                            )
                        )
                    ).map(|r| Message::SpotifyAuth(r))
                } else {
                    Task::none()
                }
            }

            Message::SpotifyAuth(res) => {

                println!("[SPOTIFY] Authentication received: IS_OK: {}", res.is_ok());

                match res {
                    Ok(creds) => self.spotify_credentials = Some(creds),
                    Err(_) => {
                        if let Some(page) = self.page.as_mut() {
                            let _ = page.update(Message::SpotifyAuthenticationFailedAgain);
                        }
                        eprintln!("[SPOTIFY] Authentication Failed");
                        return Task::none();
                    }
                }

                if let Some(page) = self.page.as_mut() {
                    let _ = page.update(Message::SpotifyAuthenticationSuccess);
                }

                Task::none()
            }

            Message::SpotifyPlaylist(uri) => {

                if let Some(page) = self.page.as_mut() {
                    // Let the page know the search has been received
                    let _ = page.update(Message::SpotifyPlaylist(String::new()));
                }

                let id = if uri.len() != 11 {
                    match crate::backend::spotify::extract_spotify_playlist_id(uri.clone()) {
                        Some(id) => id,
                        None => uri
                    }
                } else {
                    uri
                };

                if let Some(creds) = self.spotify_credentials.as_ref() {
                    println!("[SPOTIFY] Received creds, spawning stream");
                    Task::stream(SpotifySongStream::new(id, creds.clone())).map(
                        |item| match item {
                            SpotifyEmmision::PlaylistItem(item) => Message::SpotifyPlaylistItem(item),
                            SpotifyEmmision::PlaylistName(name, size) => Message::SpotifyPlaylistName(name, size),
                            SpotifyEmmision::PlaylistIDFailure => Message::SpotifyInvalidID
                        }
                    )
                } else {
                    println!("[SPOTIFY] Authentication failed when trying to make stream");
                    Task::done(Message::SpotifyAuthFailed)
                }
            }

            Message::SpotifyPlaylistItem(item) => {
                let track = match item.track {
                    Some(track) => match track {
                        PlayableItem::Track(track) => track,
                        _ => return Task::none()
                    },
                    None => return Task::none()
                };

                match desync(&self.database).get_song_by_name_exact(
                    track.name.clone(), self.directories.get_music_ref(), self.directories.get_thumbnails_ref()
                ) {
                    Some(song) => Task::done(Message::SearchResult(song, true)),
                    None => Task::done(Message::SpotifySongToYoutube(track))
                }

            }

            Message::SpotifySongToYoutube(track) => {
                match self.directories.get_dlp_ref() {
                    Some(dlp_path) => Task::future(
                        load_spotify_song(
                            track,
                            dlp_path.to_path_buf(),
                            self.database.clone(),
                            self.directories.get_music_ref().to_owned(),
                            self.directories.get_thumbnails_ref().to_owned()
                        )
                    ).map(|res| match res {
                        Ok(song) => Message::SearchResult(song, true),
                        Err(_) => Message::None
                    }),
                    None => Task::none()
                }
            }

            Message::DownloadAll(mut songs) => {
                if songs.len() == 0 { return Task::none(); }
                let mut task = Task::done(Message::Download(songs.remove(0)));
                songs.reverse();

                while let Some(song) = songs.pop() {
                    task = task.chain(Task::done(Message::Download(song)));
                }

                task
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
            ),
            PageType::ImportSpotify => Box::new(ImportPage::new(
                self.directories.clone(),
                self.database.clone(),
                self.spotify_id.clone(),
                self.spotify_secret.clone()
            ))
        });
    }
}
