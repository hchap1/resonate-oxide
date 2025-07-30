#![windows_subsystem = "windows"]

mod frontend;
mod backend;

use backend::audio::AudioTask;
use backend::database_manager::Database;
use backend::filemanager::DataDir;
use backend::mediacontrol::MediaControl;
use backend::mediacontrol::MediaPacket;
use backend::util::Relay;
use iced::Task;

use frontend::application::Application;
use frontend::message::Message;

fn main() -> Result<(), iced::Error> {

    let dir = DataDir::create_or_load().expect("FATAL: Could not create directory.");
    let database = Database::new(dir.get_root_ref().to_path_buf());
    let (_media_control, receiver) = MediaControl::new(dir.get_root_ref().to_path_buf());

    iced::daemon("Resonate-Oxide", Application::update, Application::view)
        .run_with(|| (Application::new(dir, database), Task::batch(vec![
            Message::LoadAudio.task(),
            Message::DownloadDLP.task(),
            Message::LoadSecrets.task(),
            Message::LoadAllPlaylists.task(),
            Message::LoadEverythingIntoQueue.task(),
            Task::stream(
                Relay::consume_receiver(
                    receiver,
                    |v| match v {
                        MediaPacket::TogglePlayback => Some(Message::AudioTask(AudioTask::TogglePlayback)),
                        MediaPacket::SkipForward => Some(Message::AudioTask(AudioTask::SkipForward)),
                        MediaPacket::SkipBackward => Some(Message::AudioTask(AudioTask::SkipBackward)),
                        _ => Some(Message::None)
                    }
                )
            ),
        ])))
}
