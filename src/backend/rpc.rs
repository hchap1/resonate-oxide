const CLIENT_ID: &str = "1383062372746264648";

use std::thread::JoinHandle;
use std::thread::spawn;

use async_channel::Sender;
use async_channel::Receiver;
use async_channel::unbounded;

#[derive(Debug)]
pub enum RPCError {
    ChannelDied,
    Failed
}

#[derive(Debug, Clone)]
pub enum RPCMessage {
    SetStatus(Song)
}

pub struct RPCManager {
    _handle: JoinHandle<Result<(), RPCError>>,
    sender: Sender<RPCMessage>
}

impl RPCManager {
    pub fn new() -> RPCManager {
        let (sender, receiver) = unbounded();
        Self {
            _handle: spawn(|| rpc_thread(receiver)),
            sender
        }
    }

    pub fn send(&self, message: RPCMessage) {
        let _ = self.sender.send(message);
    }
}

use discord_rich_presence::activity::Activity;
use discord_rich_presence::DiscordIpc;
use discord_rich_presence::DiscordIpcClient;
use crate::backend::music::Song;

fn rpc_thread(receiver: Receiver<RPCMessage>) -> Result<(), RPCError> {
    let mut drpc = match DiscordIpcClient::new(CLIENT_ID) {
        Ok(drpc) => drpc,
        Err(_) => return Err(RPCError::Failed)
    };

    if drpc.connect().is_err() { return Err(RPCError::Failed) }

    loop {
        let message = match receiver.recv() {
            Ok(message) => message,
            Err(_) => return Err(RPCError::ChannelDied)
        };

        match message {
            RPCMessage::SetStatus(song) => {
                let message = format!("Listening to {} by {}", song.title, song.artist);
                let _ = drpc.set_activity(
                    Activity::new()
                        .state(message.as_str())
                        .activity_type(discord_rich_presence::activity::ActivityType::Listening)
                );
            }
        }
    }
}
