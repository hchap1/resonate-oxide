use std::thread::JoinHandle;
use std::thread::spawn;
use std::sync::mpsc::Sender;
use std::sync::mpsc::Receiver;
use std::sync::mpsc::channel;
use std::io::Cursor;
use std::io::Read;
use std::fs::File;

use rodio::Decoder;
use rodio::OutputStream;
use rodio::OutputStreamHandle;
use rodio::Sink;

use crate::backend::music::Song;
use crate::backend::error::ResonateError;

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

fn audio_thread(sink: Sink, task_downstream: Receiver<AudioTask>, queue_upstream: Sender<Queue>) {

    let mut queue: Queue = Queue::new();

    loop {
        match task_downstream.try_recv() {
            Ok(task) => match task {
                AudioTask::Play => sink.play(),
                AudioTask::Pause => sink.pause(),
                AudioTask::TogglePlayback => if sink.is_paused() { sink.play() } else { sink.pause() },
                AudioTask::SkipForward => if queue.position < queue.songs.len() - 1 { queue.position += 1 },
                AudioTask::SkipBackward => if queue.position > 0 { queue.position -= 1 },
                AudioTask::Push(song) => {
                    if let Some(queue_item) = QueueItem::new(song) {
                        queue.songs.push(queue_item);
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
            },
            Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                eprintln!("[AUDIO] Task channel became unresponsive. Killing thread.");
                return;
            },
            _ => {}
        }

        if sink.empty() && queue.songs.len() == 0 {
            if queue.position < queue.songs.len() - 1 {
                queue.position += 1;
            } else {
                continue;
            }

            let data = match Decoder::new(queue.songs[queue.position].audio.clone()) {
                Ok(data) => data,
                Err(_) => continue
            };

            sink.clear();
            sink.append(data);
        }
    }
}

pub struct AudioPlayer {
    _stream: OutputStream,
    _handle: OutputStreamHandle,
    _thread_handle: JoinHandle<()>,

    task_upstream: Sender<AudioTask>,
    queue_downstream: Receiver<Queue>
}

impl AudioPlayer {
    pub fn new() -> Result<AudioPlayer, ResonateError> {

        let (task_upstream, task_downstream) = channel::<AudioTask>();
        let (queue_upstream, queue_downstream) = channel::<Queue>();

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
            queue_downstream
        })
    }
}
