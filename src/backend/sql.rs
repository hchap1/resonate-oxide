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

pub const INSERT_SONG: &str = "
    INSERT INTO Songs
    VALUES(null, ?, ?, ?, ?, ?)
";

pub const INSERT_PLAYLIST: &str = "
    INSERT INTO Playlists
    VALUES(null, ?)
";

pub const INSERT_ENTRY: &str = "
    INSERT INTO Entries
    VALUES(?, ?)
";

pub const SELECT_ALL_SONGS_IN_PLAYLIST: &str = "
    SELECT Songs.* FROM Songs
    INNER JOIN Entries ON Songs.id = Entries.song_id
    WHERE Entries.playlist_id = ?;
";

pub const INSERT_SECRET: &str = "
    INSERT INTO Secrets
    VALUES(?, ?)
";

pub const REMOVE_SONG_FROM_PLAYLIST: &str = "DELETE FROM Entries WHERE song_id = ? AND playlist_id = ?";
pub const REMOVE_ALL_FROM_PLAYLIST: &str = "DELETE FROM Entries WHERE playlist_id = ?";
pub const REMOVE_PLAYLIST: &str = "DELETE FROM Playlists WHERE id = ?";
pub const SELECT_SONG_BY_YOUTUBE_ID: &str = "SELECT * FROM Songs WHERE yt_id = ?";
pub const SELECT_ALL_SONGS: &str = "SELECT * FROM Songs";
pub const UPDATE_PLAYLIST_NAME: &str = "UPDATE Playlists SET title = ? WHERE id = ?";
pub const SELECT_PLAYLIST_BY_ID: &str = "SELECT * FROM Playlists WHERE id = ?";
pub const SELECT_ALL_PLAYLISTS: &str = "SELECT * FROM Playlists";
pub const SELECT_SONG_BY_TITLE: &str = "SELECT * FROM Songs WHERE title = ?";
pub const SELECT_SECRET_BY_NAME: &str = "SELECT * FROM Secrets WHERE name = ?";
pub const REMOVE_SECRET_BY_NAME: &str = "DELETE FROM Secrets WHERE name = ?";
