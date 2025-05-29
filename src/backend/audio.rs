use std::thread::JoinHandle;
use std::thread::spawn;
use std::thread::sleep;
use std::sync::mpsc::Sender;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::channel;
use std::io::Cursor;
use std::io::Read;
use std::fs::File;
use std::time::Duration;

use rodio::Decoder;
use rodio::OutputStream;
use rodio::OutputStreamHandle;
use rodio::Sink;

use async_channel::unbounded;

use crate::backend::music::Song;
use crate::backend::error::ResonateError;
use crate::frontend::message::Message;

pub struct QueueFramework {
    pub songs: Vec<Song>,
    pub position: usize,
    pub playing: bool
}

pub struct QueueItem {
    song: Song,
    audio: Cursor<Vec<u8>>
}

impl QueueItem {
    pub fn new(song: Song) -> Option<QueueItem> {

        let path = match song.music_path.as_ref() {
            Some(path) => path.as_path(),
            None => return None
        };

        let mut file = match File::open(path) {
            Ok(file) => file,
            Err(_) => return None
        };

        let mut buf = Vec::new();

        match file.read_to_end(&mut buf) {
            Ok(_) => {},
            Err(_) => return None
        };

        Some(QueueItem {
            song,
            audio: Cursor::new(buf)
        })
    }
}

pub struct Queue {
    songs: Vec<QueueItem>,
    position: usize
}

impl Queue {
    pub fn new() -> Queue {
        Queue {
            songs: Vec::new(),
            position: 0
        }
    }
}

#[derive(Debug, Clone)]
pub enum AudioTask {
    TogglePlayback,
    Play,
    Pause,
    SkipForward,
    SkipBackward,
    Push(Song),
    Insert(Song),
    EndThread
}

fn load_audio(sink: &Sink, queue: &Queue, queue_upstream: &async_channel::Sender<Message>) {
    // assume position has already been adjusted
    if let Some(queue_item) = queue.songs.get(queue.position) {
        let decoder = match Decoder::new(queue_item.audio.clone()) {
            Ok(decoder) => decoder,
            Err(_) => return
        };

        sink.clear();
        sink.append(decoder);
        sink.play();

        let _ = queue_upstream.send(Message::QueueUpdate(
            QueueFramework {
                songs: queue.songs.iter().map(|qi| qi.song.clone()).collect(),
                position: queue.position,
                playing: !sink.is_paused()
            }
        ));
    }
}

fn audio_thread(
    sink: Sink, task_downstream: Receiver<AudioTask>,
    queue_upstream: async_channel::Sender<Message>
) {
    let mut queue: Queue = Queue::new();

    loop {
        sleep(Duration::from_millis(500));

        println!("[AUDIO] Trying to recv.");
        match task_downstream.try_recv() {
            Ok(task) => {
                println!("[AUDIO] Received.");
                match task {
                    AudioTask::Play => sink.play(),
                    AudioTask::Pause => sink.pause(),
                    AudioTask::TogglePlayback => if sink.is_paused() { sink.play() } else { sink.pause() },

                    AudioTask::SkipForward => {
                        if queue.position < queue.songs.len() - 1 { queue.position += 1 }
                        load_audio(&sink, &queue, &queue_upstream);
                    }

                    AudioTask::SkipBackward => {
                        if queue.position > 0 { queue.position -= 1 }
                        load_audio(&sink, &queue, &queue_upstream);
                    }
                    
                    AudioTask::Push(song) => {
                        if let Some(queue_item) = QueueItem::new(song) {
                            queue.songs.push(queue_item);
                        } else {
                        }
                    }
                    AudioTask::Insert(song) => {
                        if let Some(queue_item) = QueueItem::new(song) {
                            queue.songs.insert(0, queue_item);
                        }

                        // Offsets all the other songs, thus account for this
                        if queue.songs.len() != 1 {
                            queue.position += 1;
                        }
                    },
                    AudioTask::EndThread => return
                }
            },
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                eprintln!("[AUDIO] Task channel became unresponsive. Killing thread.");
                return;
            },
            _ => {}
        }

        if sink.empty() && queue.songs.len() != 0 {
            if queue.position < queue.songs.len() - 1 {
                queue.position += 1;
            }
            load_audio(&sink, &queue, &queue_upstream);
        }
    }
}

pub struct AudioPlayer {
    _stream: OutputStream,
    _handle: OutputStreamHandle,
    _thread_handle: JoinHandle<()>,

    task_upstream: Sender<AudioTask>,
    queue_downstream: Option<async_channel::Receiver<Message>>
}

impl AudioPlayer {
    pub fn new() -> Result<AudioPlayer, ResonateError> {

        let (task_upstream, task_downstream) = channel::<AudioTask>();
        let (queue_upstream, queue_downstream) = unbounded::<Message>();

        let (_stream, handle) = match OutputStream::try_default() {
            Ok(data) => data,
            Err(_) => return Err(ResonateError::AudioStreamError)
        };

        let sink = match Sink::try_new(&handle) {
            Ok(sink) => sink,
            Err(_) => return Err(ResonateError::AudioStreamError)
        };

        let _thread_handle = spawn(move || audio_thread(sink, task_downstream, queue_upstream));

        Ok(AudioPlayer {
            _stream,
            _handle: handle,
            _thread_handle,
            task_upstream,
            queue_downstream: Some(queue_downstream)
        })
    }

    pub fn send_task(&self, task: AudioTask) -> Result<(), ()> {
        match self.task_upstream.send(task) {
            Ok(_) => Ok(()),
            Err(e) => {
                println!("[AUDIO] Error sending task: {e:?}");
                Err(())
            }
        }
    }

    pub fn take_queue_stream(&mut self) -> Option<async_channel::Receiver<Message>> {
        self.queue_downstream.take()
    }
}
