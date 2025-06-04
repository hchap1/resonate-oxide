mod frontend;
mod backend;

use iced::Task;

use dotenvy::dotenv;
use std::env;

use frontend::application::Application;
use frontend::message::Message;

fn main() -> Result<(), iced::Error> {
    dotenv().ok();

    let client_id = env::var("CLIENT_ID").ok();
    let client_secret = env::var("CLIENT_SECRET").ok();

    let fm_username = env::var("FM_USERNAME").ok();
    let fm_password = env::var("FM_PASSWORD").ok();

    let fm_key = env::var("FM_KEY").ok();
    let fm_secret = env::var("FM_SECRET").ok();

    iced::application("Resonate-Oxide", Application::update, Application::view)
        .run_with(|| (Application::default(), Task::batch(vec![
            Task::done(Message::LoadAudio),
            Task::done(Message::DownloadDLP),
            Task::done(Message::SpotifyCreds(
                client_id, client_secret
            )),

        ])))
}
