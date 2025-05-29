mod frontend;
mod backend;

use iced::Task;

use frontend::application::Application;
use frontend::message::Message;

fn main() -> Result<(), iced::Error> {
    // TODO: swap temporary repeat button for actual repeat icon
    iced::application("Resonate-Oxide", Application::update, Application::view)
        .run_with(|| (Application::default(), Task::batch(vec![
            Task::done(Message::LoadAudio),
            Task::done(Message::DownloadDLP),
        ])))
}
