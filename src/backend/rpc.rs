const CLIENT_ID: u64 = 1383062372746264648;

use std::thread::JoinHandle;
use std::thread::spawn;

use crossbeam_channel::Sender;
use crossbeam_channel::Receiver;
use crossbeam_channel::unbounded;

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
    handle: JoinHandle<Result<(), RPCError>>,
    sender: Sender<RPCMessage>
}

impl RPCManager {
    pub fn new() -> RPCManager {
        let (sender, receiver) = unbounded();
        Self {
            handle: spawn(|| rpc_thread(receiver)),
            sender
        }
    }

    pub fn send(&self, message: RPCMessage) {
        let _ = self.sender.send(message);
    }
}

use discord_rpc_client::Client;
use crate::backend::music::Song;

fn rpc_thread(receiver: Receiver<RPCMessage>) -> Result<(), RPCError> {
    let mut drpc = Client::new(CLIENT_ID);
    drpc.start();

    loop {
        let message = match receiver.recv() {
            Ok(message) => message,
            Err(_) => return Err(RPCError::ChannelDied)
        };

        match message {
            RPCMessage::SetStatus(song) => {
                match drpc.set_activity(|act| {
                    act.state(
                        format!("Listening to {} by {}", song.title, song.artist)
                    ).details(
                        "On Resonate-Oxide"
                    )
                }) {
                    Ok(_) => {}, Err(_) => return Err(RPCError::Failed)
                }
            }
        }
    }
}
