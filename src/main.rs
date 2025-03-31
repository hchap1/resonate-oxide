mod frontend;
mod backend;

use backend::{database::Database, filemanager::DataDir};
use frontend::application::Application;
use iced::Task;

#[tokio::main]
async fn main() -> Result<(), iced::Error> {
    let datadir: DataDir = match DataDir::create_or_load() {
        Ok(datadir) => datadir,
        Err(_) => panic!("Couldn't create directory.")
    };

    let database: Database = match Database::new(datadir.get_root_ref()) {
        Ok(database) => database,
        Err(_) => panic!("Couldn't create directory.")
    };

    iced::application("Resonate-Oxide", Application::update, Application::view).run()
}
