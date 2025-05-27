mod frontend;
mod backend;

use iced::Task;

use frontend::application::Application;
use frontend::message::Message;

#[tokio::main]
async fn main() -> Result<(), iced::Error> {
    iced::application("Resonate-Oxide", Application::update, Application::view)
        .run_with(|| (Application::default(), Task::done(Message::DownloadDLP)))
}
