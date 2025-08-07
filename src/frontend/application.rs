use std::collections::HashSet;

use iced::Element;
use iced::futures::FutureExt;
use iced::alignment::Vertical;
use iced::widget::Column;
use iced::widget::Row;
use iced::Length;
use iced::Task;

use rspotify::model::PlayableItem;

use rust_fm::auth::WebOAuth;
use rust_fm::playing::Scrobble;
use rust_fm::token::WebCallback;
use rust_fm::session::WebSession;
use rust_fm::playing::NowPlaying;

use crate::backend::lyrics::Lyrics;
use crate::frontend::tray::SimpleTray;
use crate::frontend::message::Message;
use crate::frontend::message::PageType;
use crate::frontend::widgets::ResonateWidget;
use crate::frontend::pages::playlist_page::PlaylistPage;
use crate::frontend::pages::import_page::ImportPage;
use crate::frontend::pages::settings_page::SettingsPage;
use crate::frontend::pages::search_page::SearchPage;
use crate::frontend::pages::playlists_page::PlaylistsPage;

use crate::backend::util::is_song_similar;
use crate::backend::database_interface::DatabaseInterface;
use crate::backend::audio::AudioTask;
use crate::backend::audio::ProgressUpdate;
use crate::backend::audio::QueueFramework;
use crate::backend::audio::ScrobbleRequest;
use crate::backend::filemanager::install_dlp;
use crate::backend::music::Song;
use crate::backend::settings::Secret;
use crate::backend::rpc::RPCManager;
use crate::backend::rpc::RPCMessage;
use crate::backend::settings::Settings;
use crate::backend::spotify::SpotifySongStream;
use crate::backend::util::Relay;
use crate::backend::spotify::try_auth;
use crate::backend::spotify::load_spotify_song;
use crate::backend::spotify::SpotifyEmmision;
use crate::backend::web::download_song;
use crate::backend::thumbnail::ThumbnailManager;
use crate::backend::filemanager::DataDir;
use crate::backend::database_manager::Database;
use crate::backend::audio::AudioPlayer;
use crate::backend::mediacontrol::MediaControl;

pub trait Page {
    fn update(&mut self, message: Message) -> Task<Message>;
    fn view(&self, current_song_downloads: &HashSet<String>, queued_downloads: &HashSet<Song>) -> Column<'_, Message>;
    fn back(&self, previous_page: (PageType, Option<usize>)) -> (PageType, Option<usize>);
}

pub struct Application<'a> {
    settings: Settings,
    page: Box<dyn Page + 'a>,
    directories: DataDir,
    database: Database,
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
    spotify_secret: Option<String>,
    last_fm_auth: Option<WebOAuth>,
    rpc_manager: RPCManager,
    _mediacontroller: Option<MediaControl>,
    tray: SimpleTray,
    lyrics_backend: Option<Lyrics>,
    lyrics: Option<String>,
    show_lyrics: bool
}

impl Default for Application<'_> {
    fn default() -> Self {
        let datadir = match DataDir::create_or_load() {
            Ok(datadir) => datadir,
            Err(_) => panic!("Couldn't create suitable directory location")
        };

        let database = Database::new(datadir.get_root_ref().to_path_buf());
        Self::new(datadir, database)
    }
}

impl Application<'_> {
    pub fn new(directories: DataDir, database: Database) -> Self {

        Self {
            settings: Settings::default(),
            page: Box::new(PlaylistsPage::new(database.derive())),
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
            spotify_secret: None,
            last_fm_auth: None,
            rpc_manager: RPCManager::new(),
            _mediacontroller: None,
            tray: SimpleTray::new(),
            lyrics_backend: Lyrics::new(),
            lyrics: None,
            show_lyrics: false
        }
    }

    pub fn view(&self, _: iced::window::Id) -> Element<'_, Message> {
        ResonateWidget::window(
            Column::new().spacing(20).push(
                Row::new().spacing(20).push(
                    Column::new().spacing(20)
                        .push(
                            if !self.show_lyrics {
                                self.page.view(&self.current_song_downloads, &self.download_queue)
                            } else {
                                Column::new().push(match self.lyrics.as_ref() {
                                    Some(lyrics) => ResonateWidget::lyrics(lyrics),
                                    None => ResonateWidget::padded_scrollable(iced::widget::text("No Lyrics").into()).into()
                                })
                            }
                        ).width(Length::FillPortion(3))
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
            ).push(ResonateWidget::control_bar(
                self.queue_state.as_ref(), 
                self.page.back(self.last_page.clone()),
                self.progress_state,
                self.volume,
                self.directories.get_default_thumbnail(),
                &self.default_queue,
                self.show_lyrics
            ))
            .into()
        )
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {

        match message {

            Message::ToggleLyrics(val) => {
                self.show_lyrics = val;
                Task::none()
            }

            Message::Lyrics(lyric_msg) => {
                match lyric_msg {
                    super::message::lyric::LyricMsg::SpawnCollector => {
                        if let Some(engine) = self.lyrics_backend.as_mut() {
                            if let Some(receiver) = engine.take_receiver() {
                                return Task::stream(
                                    Relay::consume_receiver(
                                        receiver,
                                        |string| Some(Message::Lyrics(super::message::lyric::LyricMsg::ReturnLyrics(string)))
                                    )
                                );
                            }
                        }
                    },
                    super::message::lyric::LyricMsg::RequestLyrics(song) => {
                        if let Some(engine) = self.lyrics_backend.as_ref() {
                            engine.send(song);
                        }
                    },
                    super::message::lyric::LyricMsg::ReturnLyrics(lyrics) => {
                        self.lyrics = Some(lyrics);
                    }
                }

                Task::none()
            }

            Message::MakeTables => {
                Task::future(DatabaseInterface::create_tables(self.database.derive())).map(|_| Message::None)
            }

            Message::StartTray => {
                match self.tray.take_receiver() {
                    Some(receiver) => Task::stream(
                        Relay::consume_receiver(
                            receiver,
                            |msg| match msg {
                                super::tray::TrayMessage::OpenMain => Some(Message::OpenMain),
                                super::tray::TrayMessage::Quit => Some(Message::Quit)
                            }
                        )
                    ),
                    None => Task::none()
                }
            }

            Message::Quit => {
                iced::exit()
            }

            Message::OpenMain => {
                iced::window::open(iced::window::Settings::default()).1.map(|_| Message::None)
            }

            Message::DownloadDLP => {
                println!("[UPDATE] Check if DLP is downloaded");
                match self.directories.get_dlp_ref() {
                    Some(_) => { println!("ALREADY DOWNLOADED!"); Task::none() }
                    None => Task::future(
                        install_dlp(self.directories.get_dependencies_ref().to_path_buf())
                            .map(|res| Message::DLPDownloaded(res.ok()))
                    )
                }
            }

            Message::DLPDownloaded(dlp_path) => {
                if let Some(dlp_path) = dlp_path {
                    self.directories.take_dlp_path(dlp_path)
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
                    Task::future(
                        download_song(
                            self.directories.get_dlp_ref().map(|x| x.to_path_buf()),
                            self.directories.get_music_ref().to_path_buf(),
                            song
                        )
                    ).map(move |res| match res {
                        Ok(song) => Message::SongDownloaded(song),
                        Err(song) => Message::DownloadFailed(song)
                    })
                }
            }

            Message::SongDownloaded(song) => {
                self.current_song_downloads.remove(&song.yt_id);
                let _ = self.page.update(Message::SongDownloaded(song));

                if !self.download_queue.is_empty() {
                    let song = match self.download_queue.iter().nth(0) {
                        Some(song) => song.clone(),
                        None => return Task::none()
                    };
                    let _ = self.download_queue.remove(&song);
                    Message::Download(song).task()
                } else {
                    Task::none()
                }
            }

            Message::SearchResult(song, from_online) => {

                let search_string = song.album.as_ref().unwrap_or(&song.yt_id);
                let require_thumbnail_download = song.thumbnail_path.is_none()
                    && !self.current_thumbnail_downloads.contains(search_string);
                let _ = self.page.update(Message::SearchResult(song.clone(), from_online));

                if require_thumbnail_download {
                    self.current_thumbnail_downloads.insert(search_string.clone());
                    Task::future(
                        ThumbnailManager::download_thumbnail(
                            self.directories.get_dlp_ref().expect("DLP not installed").to_path_buf(),
                            self.directories.get_thumbnails_ref().to_path_buf(),
                            song
                        )
                    ).map(|res| match res {
                            Ok(_) => Message::UpdateThumbnails, _ => Message::None
                    })
                } else { Task::none() }
            }

            Message::MultiSearchResult(songs, is_online) => {
                Task::batch(songs.into_iter().map(|song| Message::SearchResult(song, is_online).task()))
            }

            Message::LoadPage(page_type, playlist_id) => {
                let task = match &page_type {
                    PageType::Playlists => Task::batch(vec![
                        Message::LoadAllPlaylists.task()
                    ]),
                    PageType::SearchSongs => match playlist_id {
                        Some(playlist_id) => 
                            Task::batch(vec![
                                Task::future(
                                    DatabaseInterface::get_playlist_by_id(
                                        self.database.derive(),
                                        playlist_id
                                    )
                                ).map(|playlist| match playlist {
                                        Some(playlist) => Message::PlaylistData(playlist),
                                        None => Message::None
                                }),
                                Task::stream(
                                    Relay::consume_receiver(
                                        DatabaseInterface::select_all_songs(self.database.derive()),
                                        |item_stream| match item_stream {
                                            crate::backend::database_manager::ItemStream::Error => None,
                                            crate::backend::database_manager::ItemStream::End => None,
                                            crate::backend::database_manager::ItemStream::Value(row) => {
                                                Some(Message::RowIntoSearchResult(row, None))
                                            }
                                        }
                                    )
                                )
                            ]),
                        None => Task::none()
                    },
                    PageType::ViewPlaylist => match playlist_id {
                        Some(playlist_id) => Task::batch(vec![
                            Task::future(
                                DatabaseInterface::get_playlist_by_id(
                                    self.database.derive(),
                                    playlist_id
                                )
                            ).map(|playlist| match playlist {
                                Some(playlist) => Message::PlaylistData(playlist),
                                None => Message::None
                            }),
                            Task::stream(
                                Relay::consume_receiver(
                                    DatabaseInterface::select_all_songs_in_playlist(
                                        self.database.derive(), playlist_id
                                    ), |item| match item {
                                        crate::backend::database_manager::ItemStream::Value(v) => {
                                            Some(Message::RowIntoSong(v))
                                        },
                                        crate::backend::database_manager::ItemStream::End => {
                                            None
                                        },
                                        crate::backend::database_manager::ItemStream::Error => {
                                            None
                                        }
                                    }
                                )
                            )
                        ]),
                        None => Task::none()
                    },
                    PageType::Settings => {
                        let mut tasks = Vec::new();
                        if let Some(fm_secrets) = self.last_fm_auth.as_ref() {
                            if let Some(key) = fm_secrets.get_key() {
                                tasks.push(Message::ChangeSecret(
                                    Secret::FMKey(String::from(key))
                                ).task());
                            }
                            if let Some(secret) = fm_secrets.get_secret() {
                                tasks.push(Message::ChangeSecret(
                                    Secret::FMSecret(String::from(secret))
                                ).task());
                            }
                            if let Some(session) = fm_secrets.get_session() {
                                tasks.push(Message::ChangeSecret(
                                    Secret::FMSession(String::from(session))
                                ).task());
                            }
                        }

                        if let Some(id) = self.spotify_id.as_ref() {
                            tasks.push(Message::ChangeSecret(Secret::SpotifyID(id.to_string())).task())
                        }

                        if let Some(secret) = self.spotify_secret.as_ref() {
                            tasks.push(Message::ChangeSecret(Secret::SpotifySecret(secret.to_string())).task())
                        }

                        Task::batch(tasks)
                    },
                    _ => Task::none()
                };

                self.load_page(page_type, playlist_id);
                task
            },

            Message::AddSongToPlaylist(song, playlist_id) => {
                DatabaseInterface::insert_playlist_entry(
                    self.database.derive(), song.id, playlist_id
                );
                Task::done(
                    Message::SongAddedToPlaylist(song.id)
                )
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

            Message::LoadEverythingIntoQueue => {
                Task::stream(
                    Relay::consume_receiver(
                        DatabaseInterface::select_all_songs(self.database.derive()),
                        |item_stream| match item_stream {
                            crate::backend::database_manager::ItemStream::Error => None,
                            crate::backend::database_manager::ItemStream::End => None,
                            crate::backend::database_manager::ItemStream::Value(row) => {
                                Some(Message::RowIntoSongForQueue(row))
                            }
                        }
                    )
                )
            }
            
            Message::LoadAudio => {
                let (audio_player, queue_receiver, progress_receiver, scrobble_receiver) = match AudioPlayer::new() {
                    Ok(data) => data,
                    Err(_) => return Task::none()
                };

                self.audio_player = Some(audio_player);
                Task::batch(vec![
                    Task::stream(
                        Relay::consume_receiver(
                            queue_receiver, |message| Some(Message::QueueUpdate(message))
                        )
                    ),
                    Task::stream(
                        Relay::consume_receiver(
                            progress_receiver, |message| Some(Message::ProgressUpdate(message))
                        )
                    ),
                    Task::stream(
                        Relay::consume_receiver(
                            scrobble_receiver, |message| Some(Message::ScrobbleRequest(message))
                        )
                    )
                ])
            }

            Message::RowIntoSongForQueue(row) => {
                let music_path = self.directories.get_music_ref().to_path_buf();
                let thumbnail_path = self.directories.get_thumbnails_ref().to_path_buf();
                Task::future(DatabaseInterface::construct_song(row, music_path, thumbnail_path))
                    .map(|option| match option {
                        Some(song) => Message::AudioTask(AudioTask::Push(song)),
                        None => Message::None
                    })
            }

            Message::RowIntoSong(row) => {
                Task::future(DatabaseInterface::construct_song(
                    row,
                    self.directories.get_music_ref().to_path_buf(),
                    self.directories.get_thumbnails_ref().to_path_buf()
                )).map(|song| match song {
                    Some(song) => Message::SongStream(song),
                    None => Message::None
                })
            }

            Message::RowIntoSongQuery(row, query) => {
                Task::future(DatabaseInterface::construct_song(
                    row,
                    self.directories.get_music_ref().to_path_buf(),
                    self.directories.get_thumbnails_ref().to_path_buf()
                )).map(move |option| match option {
                    Some(song) => {
                        if is_song_similar(&song, &query) > 70 { Message::SongStream(song) }
                        else { Message::None }
                    }
                    None => Message::None
                })
            }

            Message::RowIntoSearchResult(row, optn) => { Task::future(DatabaseInterface::construct_song(
                    row,
                    self.directories.get_music_ref().to_path_buf(),
                    self.directories.get_thumbnails_ref().to_path_buf()
                )).map(move |option| match option {
                    Some(song) => {
                        match &optn {
                            Some(query) => {
                                if is_song_similar(&song, query) > 70 { Message::SearchResult(song, false) }
                                else { Message::None }
                            },
                            None => Message::SearchResult(song, false)
                        }
                    }
                    None => Message::None
                })
            }

            Message::LoadEntirePlaylist(playlist_id, _) => {
                let receiver = DatabaseInterface::select_all_songs_in_playlist(self.database.derive(), playlist_id);
                Task::stream(Relay::consume_receiver(receiver,
                    |item_stream| match item_stream {
                        crate::backend::database_manager::ItemStream::End => None,
                        crate::backend::database_manager::ItemStream::Error => None,
                        crate::backend::database_manager::ItemStream::Value(row) => Some(
                            Message::RowIntoSongForQueue(row)
                        )
                   }
                ))
            }

            Message::RemoveSongFromPlaylist(song_id, playlist_id) => {
                DatabaseInterface::remove_song_from_playlist(self.database.derive(), song_id, playlist_id);
                let _ = self.page.update(Message::RemoveSongFromPlaylist(song_id, playlist_id));
                Message::AudioTask(AudioTask::RemoveSongById(song_id)).task()
            }

            Message::DeletePlaylist(playlist_id) => {
                DatabaseInterface::delete_playlist(self.database.derive(), playlist_id);
                let _ = self.page.update(Message::DeletePlaylist(playlist_id));
                Task::none()
            }

            Message::DownloadFailed(song) => {
                println!("[UPDATE] Download of {} failed.", song.title);
                self.current_song_downloads.remove(&song.yt_id);
                let _ = self.page.update(Message::DownloadFailed(song));

                if !self.download_queue.is_empty() {
                    let song = match self.download_queue.iter().nth(0) {
                        Some(song) => song.clone(),
                        None => return Task::none()
                    };
                    let _ = self.download_queue.remove(&song);
                    Message::Download(song).task()
                } else {
                    Task::none()
                }
            }

            Message::ProgressUpdate(update) => {
                self.progress_state = Some(update);
                Task::none()
            }

            Message::SpotifyCreds(id, secret) => {
                let _ = self.page.update(Message::SpotifyCreds(id.clone(), secret.clone()));

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
                    ).map(Message::SpotifyAuth)
                } else {
                    Task::none()
                }
            }

            Message::SpotifyAuth(res) => {
                match res {
                    Ok(creds) => self.spotify_credentials = Some(creds),
                    Err(_) => {
                        let _ = self.page.update(Message::SpotifyAuthenticationFailedAgain);
                        return Task::none();
                    }
                }

                let _ = self.page.update(Message::SpotifyAuthenticationSuccess);
                Task::none()
            }

            Message::SpotifyPlaylist(uri) => {
                let _ = self.page.update(Message::SpotifyPlaylist(String::new()));

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
                            SpotifyEmmision::Item(item) => Message::SpotifyPlaylistItem(item),
                            SpotifyEmmision::Name(name, size) => Message::SpotifyPlaylistName(name, size),
                            SpotifyEmmision::IDFailure => Message::SpotifyInvalidID
                        }
                    )
                } else {
                    println!("[SPOTIFY] Authentication failed when trying to make stream");
                    Message::SpotifyAuthFailed.task()
                }
            }

            Message::GetSongByTitleForSpotify(option, track) => {
                match option {
                    Some(song) => Message::SearchResult(song, true).task(),
                    None => Message::SpotifySongToYoutube(track).task()
                }
            }

            Message::SpotifyPlaylistItem(item) => {
                let track = match item.track {
                    Some(PlayableItem::Track(track)) => track,
                    _ => return Task::none()
                };

                Task::future(DatabaseInterface::select_song_by_title(
                    self.database.derive(),
                    track.name.clone(),
                    self.directories.get_music_ref().to_path_buf(),
                    self.directories.get_thumbnails_ref().to_path_buf()
                )).map(move |option| match option {
                    Some(song) => Message::GetSongByTitleForSpotify(Some(song), track.clone()),
                    None => Message::GetSongByTitleForSpotify(None, track.clone())
                })
            }

            Message::SpotifySongToYoutube(track) => {
                match self.directories.get_dlp_ref() {
                    Some(dlp_path) => Task::future(
                        load_spotify_song(
                            track,
                            dlp_path.to_path_buf(),
                            self.database.derive(),
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
                if songs.is_empty() { return Task::none(); }
                let mut task = Message::Download(songs.remove(0)).task();
                songs.reverse();

                while let Some(song) = songs.pop() {
                   task = task.chain(Message::Download(song).task());
                }
                task
            }

            Message::LoadSecrets => {
                Task::future(
                    DatabaseInterface::select_multiple_secrets(
                        self.database.derive(),
                        vec![
                            "SPOTIFY_ID".to_string(),
                            "SPOTIFY_SECRET".to_string(),
                            "FM_KEY".to_string(),
                            "FM_SECRET".to_string(),
                            "FM_SESSION".to_string()
                        ]
                    )
                ).map(|res| {
                    Message::SecretsLoaded(res)
                })
            }

            Message::SecretsLoaded(mut secrets) => {
                let fm_session = secrets.pop().unwrap();
                let fm_secret = secrets.pop().unwrap();
                let fm_key = secrets.pop().unwrap();
                let spotify_secret = secrets.pop().unwrap();
                let spotify_id = secrets.pop().unwrap();
                let ready = fm_key.is_some() && fm_secret.is_some() && fm_session.is_none();

                let fm_key_string = if let Some(Secret::FMKey(fm_key)) = fm_key {
                    Some(fm_key)
                } else { None };
                let fm_secret_string = if let Some(Secret::FMSecret(fm_secret)) = fm_secret {
                    Some(fm_secret)
                } else { None };
                let fm_session_string = if let Some(Secret::FMSession(fm_session)) = fm_session {
                    Some(fm_session)
                } else { None };

                let spotify_id_string = if let Some(Secret::SpotifyID(spotify_id)) = spotify_id {
                    Some(spotify_id)
                } else { None };
                let spotify_secret_string = if let Some(Secret::SpotifySecret(spotify_secret)) = spotify_secret {
                    Some(spotify_secret)
                } else { None };

                self.last_fm_auth = Some(
                    WebOAuth::from_key_and_secret(fm_key_string, fm_secret_string, fm_session_string)
                );

                Task::batch(vec![
                    Message::SpotifyCreds(spotify_id_string, spotify_secret_string).task(),
                    if ready { Message::FMAuthenticate.task() } else { Task::none() }
                ])
            }

            Message::SaveSecret(secret) => {
                let (n, v) = match secret {
                    Secret::SpotifyID(x) => ("SPOTIFY_ID", x),
                    Secret::SpotifySecret(x) => ("SPOTIFY_SECRET", x),
                    Secret::FMKey(x) => ("FM_KEY", x),
                    Secret::FMSecret(x) => ("FM_SECRET", x),
                    Secret::FMSession(x) => ("FM_SESSION", x),
                };

                Task::future(DatabaseInterface::insert_or_update_secret(self.database.derive(), n.to_string(), v))
                    .map(|res| match res {
                        Ok(_) => Message::LoadSecrets,
                        Err(_) => Message::None
                    })
            }

            Message::FMAuthenticate => {
                let auth = match self.last_fm_auth.take() {
                    Some(auth) => auth,
                    None => return Message::FMAuthFailed(None).task()
                };

                Task::future(WebCallback::oauth(auth)).map(
                    |(auth, res)| match res {
                        Ok(_) => {
                            Message::FMGetSession(auth)
                        },
                        Err(_) => {
                            Message::FMAuthFailed(Some(auth))
                        }
                    }
                )
            }

            Message::FMGetSession(auth) => {
                Task::future(WebSession::get(auth)).map(
                    |(auth, res)| match res {
                        Ok(_) => Message::FMAuthSuccess(auth),
                        Err(_) => Message::FMAuthFailed(Some(auth))
                    }
                )
            }

            Message::FMAuthFailed(auth) => {
                self.last_fm_auth = auth;
                Task::none()
            }

            Message::FMAuthSuccess(auth) => {
                self.last_fm_auth = Some(auth);
                Message::FMSaveSecrets.task()
            }

            Message::FMSaveSecrets => {
                let auth = match self.last_fm_auth.as_ref() {
                    Some(auth) => auth,
                    None => return Task::none()
                };

                let mut tasks = Vec::new();

                if let Some(x) = auth.get_key() {
                    tasks.push(Task::future(DatabaseInterface::insert_or_update_secret(
                        self.database.derive(), "FM_KEY".to_string(), x.to_string()
                    )))
                }
                if let Some(x) = auth.get_secret() {
                    tasks.push(Task::future(DatabaseInterface::insert_or_update_secret(
                        self.database.derive(), "FM_SECRET".to_string(), x.to_string()
                    )))
                }
                if let Some(x) = auth.get_session() {
                    tasks.push(Task::future(DatabaseInterface::insert_or_update_secret(
                        self.database.derive(), "FM_SESSION".to_string(), x.to_string()
                    )))
                }
                Task::batch(tasks.into_iter().map(|x| x.map(|_| Message::None)))
            }

            Message::FMSetNowPlaying(song) => {
                let scrobble = Scrobble::new(
                    song.title.clone(),
                    song.artist.clone(),
                    song.album.clone()
                );

                let auth = match self.last_fm_auth.clone() {
                    Some(auth) => auth,
                    None => return Task::none()
                };

                let auth_clone = auth.clone();

                let fm_task = Task::future(NowPlaying::set_now_playing(auth_clone, scrobble)).map(
                    |x| match x {
                        Ok(_) => Message::FMScrobbleSuccess,
                        Err(_) => Message::FMScrobbleFailure
                    }
                );

                self.lyrics = None;
                let rpc_task = Message::RPCMessage(RPCMessage::SetStatus(song.clone())).task();
                Task::batch(vec![
                    Message::Lyrics(super::message::lyric::LyricMsg::RequestLyrics(song)).task(),
                    rpc_task.chain(fm_task)
                ])
            }

            Message::FMPushScrobble(song) => {
                let scrobble = Scrobble::new(
                    song.title.clone(),
                    song.artist.clone(),
                    song.album.clone()
                );

                let auth = match self.last_fm_auth.clone() {
                    Some(auth) => auth,
                    None => return Task::none()
                };

                let auth_clone = auth.clone();

                Task::future(NowPlaying::push_scrobble(auth_clone, scrobble, None)).map(
                    |x| match x {
                        Ok(_) => Message::FMScrobbleSuccess,
                        Err(_) => Message::FMScrobbleFailure
                    }
                )
            }

            Message::ScrobbleRequest(scrobble_request) => {
                match scrobble_request {
                    ScrobbleRequest::NowPlaying(song) => Message::FMSetNowPlaying(song).task(),
                    ScrobbleRequest::Scrobble(song) => Message::FMPushScrobble(song).task()
                }
            }

            Message::RPCMessage(message) => {
                self.rpc_manager.send(message);
                Task::none()
            }

            Message::LoadAllPlaylists => {
                Task::stream(
                    Relay::consume_receiver(
                        DatabaseInterface::select_all_playlists(self.database.derive()),
                        |item| match item {
                            crate::backend::database_manager::ItemStream::Value(v) => {
                                DatabaseInterface::construct_playlist(v).map(Message::PlaylistLoaded)
                            },
                            crate::backend::database_manager::ItemStream::End => {
                                None
                            }
                            crate::backend::database_manager::ItemStream::Error => {
                                None
                            }
                       }
                    )
                )
            }

            other => {
                self.page.update(other)
            }
        }
    }

    fn load_page(&mut self, page_type: PageType, playlist_id: Option<usize>) {
        self.last_page = self.current_page.to_owned();
        self.current_page = (page_type.clone(), playlist_id);

        self.page = match page_type {

            PageType::SearchSongs => Box::new(
                SearchPage::new(self.directories.clone(), self.database.derive(), playlist_id.unwrap())
            ),
            PageType::Playlists => Box::new(PlaylistsPage::new(self.database.derive())),

            PageType::ViewPlaylist => Box::new(
                match PlaylistPage::new(playlist_id, self.database.derive(), self.directories.clone()) {
                    Ok(page) => page,
                    Err(_) => return // THIS SHOULD BE AN ERROR NOTIFICATION
                }
            ),

            PageType::ImportSpotify => Box::new(ImportPage::new(
                self.directories.clone(),
                self.database.derive(),
                self.spotify_id.clone(),
                self.spotify_secret.clone()
            )),

            PageType::Settings => {
                Box::new(SettingsPage::new())
            }
        };
    }
}
