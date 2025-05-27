use std::collections::HashSet;

use iced::Element;
use iced::widget::text;
use iced::Task;

// GUI PAGES
use crate::frontend::search_page::SearchPage;
use crate::frontend::playlists::PlaylistsPage;
use crate::frontend::playlist_page::PlaylistPage;

use crate::frontend::message::Message;
use crate::frontend::message::PageType;
use crate::frontend::backend_interface::async_download_thumbnail;

use crate::backend::filemanager::DataDir;
use crate::backend::database::Database;
use crate::backend::util::{sync, AM};

use super::backend_interface::async_download_song;
use super::backend_interface::async_install_dlp;

pub trait Page {
    fn update(&mut self, message: Message) -> Task<Message>;
    fn view(&self, current_song_downloads: &HashSet<String>) -> Element<'_, Message>;
}

pub struct Application<'a> {
    page: Option<Box<dyn Page + 'a>>,
    directories: DataDir,
    database: AM<Database>,

    current_thumbnail_downloads: HashSet<String>,
    current_song_downloads: HashSet<String>
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

        for song in database.retrieve_all_songs(datadir.get_music_ref(), datadir.get_thumbnails_ref()) {
            println!("Song: {song}");
        }

        let unique = database.is_unique(&String::from("9qnqYL0eNNI"));
        println!("{unique}");

        println!("Root: {:?}", datadir.get_root_ref());

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
            current_song_downloads: HashSet::new()
        }
    }

    pub fn view(&self) -> Element<'_, Message> {
        match self.page.as_ref() {
            Some(page) => page.view(&self.current_song_downloads),
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

                println!("DOWNLOAD TASK RECEIVED FOR {}", song.title);
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
                    Message::Download(song)
                )
            }

            other => match self.page.as_mut() {
                Some(page) => page.update(other),
                None => Task::none()
            }

        }
    }

    fn load_page(&mut self, page_type: PageType, playlist_id: Option<usize>) {
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
