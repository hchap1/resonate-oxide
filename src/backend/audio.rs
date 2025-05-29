use std::thread::JoinHandle;
use std::thread::spawn;
use std::thread::sleep;
use std::io::Cursor;
use std::io::Read;
use std::fs::File;
use std::time::Duration;

use crossbeam_channel::Receiver;
use crossbeam_channel::Sender;
use crossbeam_channel::bounded;

use rodio::Decoder;
use rodio::OutputStream;
use rodio::OutputStreamHandle;
use rodio::Sink;

use crate::backend::music::Song;
use crate::backend::error::ResonateError;

#[derive(Debug, Clone)]
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

fn update_queue(sink: &Sink, queue: &Queue, queue_upstream: &Sender<QueueFramework>) {
    let _ = queue_upstream.send(
        QueueFramework {
            songs: queue.songs.iter().map(|qi| qi.song.clone()).collect(),
            position: queue.position,
            playing: !sink.is_paused()
        }
    );
}

fn load_audio(sink: &Sink, queue: &Queue, queue_upstream: &Sender<QueueFramework>) {
    // assume position has already been adjusted
    if let Some(queue_item) = queue.songs.get(queue.position) {
        let decoder = match Decoder::new(queue_item.audio.clone()) {
            Ok(decoder) => decoder,
            Err(_) => return
        };

        sink.clear();
        sink.append(decoder);
        sink.play();
        
        update_queue(sink, queue, queue_upstream);
    }
}

fn audio_thread(
    sink: Sink, task_downstream: Receiver<AudioTask>, queue_upstream: Sender<QueueFramework>) {

    let mut queue: Queue = Queue::new();

    loop {
        sleep(Duration::from_millis(200));

        match task_downstream.try_recv() {
            Ok(task) => {
                match task {
                    AudioTask::Play => {
                        sink.play();
                        update_queue(&sink, &queue, &queue_upstream);
                    }
                    AudioTask::Pause => {
                        sink.pause();
                        update_queue(&sink, &queue, &queue_upstream);
                    }
                    AudioTask::TogglePlayback => {
                        if sink.is_paused() { sink.play() } else { sink.pause() }
                        update_queue(&sink, &queue, &queue_upstream);
                    },

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
                        }
                        update_queue(&sink, &queue, &queue_upstream);
                    }
                    AudioTask::Insert(song) => {
                        if let Some(queue_item) = QueueItem::new(song) {
                            queue.songs.insert(0, queue_item);
                        }

                        // Offsets all the other songs, thus account for this
                        if queue.songs.len() != 1 {
                            queue.position += 1;
                        }
                        update_queue(&sink, &queue, &queue_upstream);
                    },
                    AudioTask::EndThread => return
                }
            },
            Err(crossbeam_channel::TryRecvError::Disconnected) => {
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
}

impl AudioPlayer {
    pub fn new() -> Result<(AudioPlayer, Receiver<QueueFramework>), ResonateError> {

        let (task_upstream, task_downstream) = bounded::<AudioTask>(256);
        let (queue_upstream, queue_downstream) = bounded::<QueueFramework>(256);

        let (_stream, handle) = match OutputStream::try_default() {
            Ok(data) => data,
            Err(_) => return Err(ResonateError::AudioStreamError)
        };

        let sink = match Sink::try_new(&handle) {
            Ok(sink) => sink,
            Err(_) => return Err(ResonateError::AudioStreamError)
        };

        let _thread_handle = spawn(move || audio_thread(sink, task_downstream, queue_upstream));

        Ok((AudioPlayer {
            _stream,
            _handle: handle,
            _thread_handle,
            task_upstream,
        }, queue_downstream))
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
}
