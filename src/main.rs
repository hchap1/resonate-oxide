#![windows_subsystem = "windows"]

mod frontend;
mod backend;

use iced::Task;

use frontend::application::Application;
use frontend::message::Message;

fn main() -> Result<(), iced::Error> {
    iced::application("Resonate-Oxide", Application::update, Application::view)
        .run_with(|| (Application::default(), Task::batch(vec![
            Message::LoadAudio.task(),
            Message::DownloadDLP.task(),
            Message::LoadSecrets.task(),
            Message::LoadAllPlaylists.task()
        ])))
}
