use rusqlite::{Connection, Statement};

pub const CREATE_SONG_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS Songs {
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        yt_id TEXT NOT NULL,
        title TEXT NOT NULL,
        artist TEXT NOT NULL,
        album TEXT NOT NULL,
        duration INTEGER NOT NULL
    };
";

pub const CREATE_PLAYLIST_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS Playlists {
        id INTEGER PRIMARY KEY AUTOINCREMENT,
        title TEXT NOT NULL
    };
";

pub const CREATE_PLAYLIST_ENTRIES_TABLE: &str = "
    CREATE TABLE IF NOT EXISTS Entries {
        playlist_id INTEGER,
        song_id INTEGER,
        PRIMARY KEY (playlist_id, song_id),
        FOREIGN KEY (playlist_id) REFERENCES Playlists(id) ON DELETE CASCADE,
        FOREIGN KEY (song_id) REFERENCES Songs(id) ON DELETE CASCADE
    };
";

pub struct Query<'a> {
    connection: &Connection
}

impl Query<'a> {
    pub fn new(connection: &Connection) -> Self { Self { connection } }
    pub fn retrieve_all_songs(self) -> Statement<'a> { self.connection.prepare("SELECT * FROM Songs").unwrap() }
    pub fn get_song_by_field(self) -> Statement<'a> { self.connection.prepare("SELECT * FROM Songs WHERE ? = ?").unwrap() }
    pub fn get_song_by_match(self) -> Statement<'a> { self.connection.prepare("SELECT * FROM Songs WHERE title LIKE ? OR artist LIKE ? OR album LIKE ?").unwrap() }

    pub fn insert_song(self) -> Statement<'a> { 
        self.connection.prepare("
            INSERT INTO Songs
            VALUES(null, ?, ?, ?, ?, ?)
        ").unwrap()
    }
}
