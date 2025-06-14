use rusqlite::{Connection, Statement};

pub const CREATE_SONG_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS Songs (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        yt_id TEXT NOT NULL,
        title TEXT NOT NULL,
        artist TEXT NOT NULL,
        album TEXT NOT NULL,
        duration INTEGER NOT NULL
    );
";

pub const CREATE_PLAYLIST_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS Playlists (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        title TEXT NOT NULL
    );
";

pub const CREATE_PLAYLIST_ENTRIES_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS Entries (
        playlist_id INTEGER,
        song_id INTEGER,
        PRIMARY KEY (playlist_id, song_id),
        FOREIGN KEY (playlist_id) REFERENCES Playlists(id) ON DELETE CASCADE,
        FOREIGN KEY (song_id) REFERENCES Songs(id) ON DELETE CASCADE
    );
";

pub const CREATE_SECRETS_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS Secrets (
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        name TEXT NOT NULL,
        value TEXT NOT NULL
    );
";

pub struct Query<'a> {
    connection: &'a Connection
}

#[allow(dead_code)]
impl<'a> Query<'a> {
    pub fn new(connection: &'a Connection) -> Self { Self { connection } }

    pub fn get_song_by_name_exact(self) -> Statement<'a> {
        self.connection.prepare("SELECT id FROM Songs WHERE title = ?").unwrap()
    }

    pub fn check_if_yt_id_exists(self) -> Statement<'a> { self.connection.prepare("SELECT yt_id FROM Songs WHERE yt_id = ?").unwrap() }

    pub fn check_if_song_in_playlist(self) -> Statement<'a> { self.connection.prepare("SELECT * FROM Entries WHERE playlist_id = ? AND song_id = ?").unwrap() }

    pub fn retrieve_all_songs(self) -> Statement<'a> { self.connection.prepare("SELECT * FROM Songs").unwrap() }
    pub fn retrieve_all_song_yt_ids(self) -> Statement<'a> { self.connection.prepare("SELECT yt_id FROM Songs").unwrap() }
    pub fn get_song_by_id(self) -> Statement<'a> { self.connection.prepare("SELECT * FROM Songs WHERE id = ?").unwrap() }
    pub fn get_playlist_by_id(self) -> Statement<'a> { self.connection.prepare("SELECT * FROM Playlists WHERE id = ?").unwrap() }
    pub fn get_song_by_match(self) -> Statement<'a> { self.connection.prepare("SELECT * FROM Songs WHERE title LIKE ? OR artist LIKE ? OR album LIKE ?").unwrap() }
    pub fn get_all_playlists(self) -> Statement<'a> { self.connection.prepare("SELECT * FROM Playlists").unwrap() }

    pub fn remove_song_from_playlist(self) -> Statement<'a> {
        self.connection.prepare("DELETE FROM Entries WHERE song_id = ? AND playlist_id = ?").unwrap()
    }

    pub fn search_playlist(self) -> Statement<'a> {
        self.connection.prepare(
            "
            SELECT e.song_id
            FROM Entries e
            JOIN Songs s ON e.song_id = s.id
            WHERE e.playlist_id = ?1
              AND (
                s.title  LIKE ?2 OR
                s.artist LIKE ?3 OR
                s.album  LIKE ?4 
              );
            "
        ).unwrap()
    }

    pub fn insert_song(self) -> Statement<'a> { 
        self.connection.prepare("
            INSERT INTO Songs
            VALUES(null, ?, ?, ?, ?, ?)
        ").unwrap()
    }

    pub fn create_playlist(self) -> Statement<'a> {
        self.connection.prepare("
            INSERT INTO Playlists
            VALUES(null, ?)
        ").unwrap()
    }

    pub fn add_song_to_playlist(self) -> Statement<'a> {
        self.connection.prepare("
            INSERT INTO Entries
            VALUES(?, ?)
        ").unwrap()
    }

    pub fn delete_playlist(self) -> Statement<'a> {
        self.connection.prepare("DELETE FROM Playlists WHERE id = ?").unwrap()
    }

    pub fn delete_all_songs_in_playlist(self) -> Statement<'a> {
        self.connection.prepare("DELETE FROM Entries WHERE playlist_id = ?").unwrap()
    }

    pub fn get_secret_by_name(self) -> Statement<'a> {
        self.connection.prepare("SELECT value FROM Secrets WHERE name = ?").unwrap()
    }

    pub fn del_secret_by_name(self) -> Statement<'a> {
        self.connection.prepare("DELETE FROM Secrets WHERE name = ?").unwrap()
    }

    pub fn set_secret_by_name(self) -> Statement<'a> {
        self.connection.prepare("
            INSERT INTO Secrets
            VALUES(null, ?, ?)
        ").unwrap()
    }
}
