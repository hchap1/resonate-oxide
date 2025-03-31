mod frontend;

use frontend::application::Application;

#[tokio::main]
async fn main() -> Result<(), iced::Error> {
    iced::application("Resonant-Oxide", Application::update, Application::view).run()
}
