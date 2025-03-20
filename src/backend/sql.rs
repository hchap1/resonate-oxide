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
