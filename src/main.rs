mod backend;
mod frontend;

use backend::web::collect_metadata;
use backend::web::flatsearch;
use backend::filemanager::DataDir;
use backend::database::Database;

#[tokio::main]
async fn main() {
    let datadir = DataDir::create_or_load().unwrap();
    let database = Database::new(datadir.get_root_ref()).unwrap();

    let songs = flatsearch(datadir.get_dlp_ref(), datadir.get_music_ref(), datadir.get_thumbnails_ref(), String::from("linkin park"), &database).await.unwrap();
    for song in &songs {
        let song_data = collect_metadata(datadir.get_dlp_ref(), datadir.get_music_ref(), datadir.get_thumbnails_ref(), song).await.unwrap();
        println!("{song_data}");
    }
}
