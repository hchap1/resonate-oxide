mod frontend;
mod backend;

use frontend::application::Application;

#[tokio::main]
async fn main() -> Result<(), iced::Error> {
    iced::application("Resonate-Oxide", Application::update, Application::view).run()
}
