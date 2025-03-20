mod backend;

use crate::backend::filemanager::DataDir;

#[tokio::main]
async fn main() {
    let dir = DataDir::create_or_load().unwrap();
    let dlp_exe = dir.install_dlp().await.unwrap();
    println!("Downloaded: {:?}", dlp_exe);
}
