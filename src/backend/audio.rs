use std::thread::JoinHandle;
use std::thread::spawn;
use std::thread::sleep;
use std::io::Cursor;
use std::io::Read;
use std::fs::File;
use std::time::Duration;
use std::default::Default;

use async_channel::Receiver;
use async_channel::Sender;
use async_channel::bounded;

use rodio::Decoder;
use rodio::OutputStream;
use rodio::OutputStreamHandle;
use rodio::Sink;

use crate::backend::music::Song;
use crate::backend::error::ResonateError;

#[derive(Debug, Clone, Default)]
pub struct QueueFramework {
    pub songs: Vec<Song>,
    pub position: usize,
    pub playing: bool,
    pub repeat: bool,
}

pub struct QueueItem {
    song: Song,
    audio: Option<Cursor<Vec<u8>>>
}

impl QueueItem {
    pub fn new(song: Song) -> Option<Self> {
        if song.music_path.is_some() {
            Some(Self { song, audio: None })
        } else {
            None
        }
    }

    pub fn load(&mut self) {
        let path = match self.song.music_path.as_ref() {
            Some(path) => path.as_path(),
            None => return
        };

        let mut file = match File::open(path) {
            Ok(file) => file,
            Err(_) => return
        };

        let mut buf = Vec::new();

        match file.read_to_end(&mut buf) {
            Ok(_) => {},
            Err(_) => return
        };

        self.audio = Some(Cursor::new(buf));
    }
}

pub struct Queue {
    songs: Vec<QueueItem>,
    position: usize,
    repeat: bool,
}

impl Queue {
    pub fn new() -> Queue {
        Queue {
            songs: Vec::new(),
            position: 0,
            repeat: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum ScrobbleRequest {
    NowPlaying(Song),
    Scrobble(Song)
}

#[derive(Debug, Clone, Copy)]
pub enum ProgressUpdate {
    Nothing,
    Seconds(f32, f32)
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub enum AudioTask {
    TogglePlayback,
    Play,
    Pause,
    SkipForward,
    SkipBackward,
    Push(Song),
    Insert(Song),
    EndThread,
    Move(usize),
    SetQueue(Vec<Song>),
    RemoveSongById(usize),
    RemoveSongByIdx(usize),
    ToggleRepeat,
    SetVolume(f32),
    ClearQueue
}

fn update_queue(sink: &Sink, queue: &Queue, queue_upstream: &Sender<QueueFramework>) {
    let _ = queue_upstream.send_blocking(
        QueueFramework {
            songs: queue.songs.iter().map(|qi| qi.song.clone()).collect(),
            position: queue.position,
            playing: !sink.is_paused(),
            repeat: queue.repeat,
        }
    );
}

fn load_audio(
    sink: &Sink, queue: &mut Queue, queue_upstream: &Sender<QueueFramework>, scrobble_upstream: &Sender<ScrobbleRequest>
) -> Option<usize> {
    // assume position has already been adjusted
    let changed_audio = if let Some(queue_item) = queue.songs.get_mut(queue.position) {

        queue_item.load();

        let audio = match queue_item.audio.as_ref() {
            Some(audio) => audio.clone(),
            None => return None
        };

        let decoder = match Decoder::new(audio) {
            Ok(decoder) => decoder,
            Err(_) => {
                update_queue(sink, queue, queue_upstream);
                return None;
            }
        };

        sink.clear();
        sink.append(decoder);
        sink.play();

        let _ = scrobble_upstream.send_blocking(ScrobbleRequest::NowPlaying(queue_item.song.clone()));

        Some(queue_item.song.id)
    } else { None };

    update_queue(sink, queue, queue_upstream);
    changed_audio
}

fn audio_thread(
    sink: Sink, task_downstream: Receiver<AudioTask>,
    queue_upstream: Sender<QueueFramework>,
    progress_upstream: Sender<ProgressUpdate>,
    scrobble_upstream: Sender<ScrobbleRequest>
) {

    let mut queue: Queue = Queue::new();
    let mut now_playing: Option<usize> = None;
    let mut scrobble_applied = false;
    let mut first_song = true;

    loop {
        sleep(Duration::from_millis(200));

        let _ = progress_upstream.send_blocking(match queue.songs.get(queue.position) {
            Some(song) => ProgressUpdate::Seconds(
                sink.get_pos().as_secs_f32(),
                song.song.duration.as_secs_f32()
            ),
            None => ProgressUpdate::Nothing
        });

        let mut should_audio_be_reloaded = false;
        let mut do_not_skip = false;

        while let Ok(task) = task_downstream.try_recv() {
            let need_reload = match task {
                AudioTask::Play => {
                    sink.play();
                    update_queue(&sink, &queue, &queue_upstream);
                    false
                }
                AudioTask::Pause => {
                    sink.pause();
                    update_queue(&sink, &queue, &queue_upstream);
                    false
                }
                AudioTask::TogglePlayback => {
                    println!("PLAYBACK TOGGLED!");
                    if sink.is_paused() { sink.play() } else { sink.pause() }
                    update_queue(&sink, &queue, &queue_upstream);
                    false
                },

                AudioTask::SkipForward => {
                    if queue.position < queue.songs.len() - 1 { queue.position += 1 }
                    true
                }

                AudioTask::SkipBackward => {
                    if queue.position > 0 { queue.position -= 1 }
                    true
                }
                
                AudioTask::Push(song) => {
                    if let Some(queue_item) = QueueItem::new(song) {
                        queue.songs.push(queue_item);
                    }
                    update_queue(&sink, &queue, &queue_upstream);
                    false
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
                    false
                },
                AudioTask::Move(target) => {
                    if queue.position == target {
                        continue
                    }

                    if target >= queue.songs.len() {
                        continue
                    }

                    queue.position = target;
                    true
                }
                AudioTask::SetQueue(songs) => {
                    sink.clear();
                    queue.position = 0;
                    queue.songs = songs.into_iter().filter_map(QueueItem::new).collect();
                    sink.play();
                    do_not_skip = true;
                    true
                }
                AudioTask::RemoveSongById(song_id) => {
                    let idx = match queue.songs.iter().enumerate().find_map(|(i, qi)|
                        if qi.song.id == song_id { Some(i) } else { None }
                    ) {
                        Some(idx) => idx,
                        None => continue
                    };

                    if idx < queue.position {
                        queue.position -= 1;
                    }

                    queue.songs.remove(idx);
                    true
                }
                AudioTask::RemoveSongByIdx(idx) => {
                    if idx >= queue.songs.len() {
                        continue
                    }

                    if idx < queue.position {
                        queue.position -= 1;
                    }

                    queue.songs.remove(idx);
                    true
                }
                AudioTask::ToggleRepeat => {
                    queue.repeat = !queue.repeat;
                    update_queue(&sink, &queue, &queue_upstream);
                    false
                }
                AudioTask::SetVolume(volume) => {
                    sink.set_volume(volume);
                    update_queue(&sink, &queue, &queue_upstream);
                    false
                }
                AudioTask::ClearQueue => {
                    queue.songs.clear();
                    queue.position = 0;
                    first_song = true;
                    true
                }
                AudioTask::EndThread => return
            };

            if need_reload { should_audio_be_reloaded = true; }
        }

        if sink.empty() && !queue.songs.is_empty() {
            if queue.position < queue.songs.len() - 1 && !queue.repeat && !do_not_skip {
                if first_song {
                    first_song = false;
                } else {
                    queue.position += 1;
                }
            } else if queue.position == queue.songs.len() - 1 && !queue.repeat {
                queue.position = 0;
            }
            should_audio_be_reloaded = true;
        }

        if let Some(song) = queue.songs.get(queue.position) {
            if let Some(playing) = now_playing {
                if song.song.id != playing {
                    should_audio_be_reloaded = true;
                }
            }

            let seconds_in = sink.get_pos().as_secs_f32();
            let total_seconds = song.song.duration.as_secs_f32();
            let ratio = seconds_in / total_seconds;

            if !scrobble_applied && ratio > 0.2f32 {
                scrobble_applied = true;
                let _ = scrobble_upstream.send_blocking(ScrobbleRequest::Scrobble(song.song.clone()));
            }
        }
        
        if should_audio_be_reloaded {
            now_playing = load_audio(&sink, &mut queue, &queue_upstream, &scrobble_upstream);
            scrobble_applied = false;
            if now_playing.is_none() {
                queue.position = 0;
                sink.clear();
            }
        }
    }
}

pub struct AudioPlayer {
    _stream: OutputStream,
    _handle: OutputStreamHandle,
    _thread_handle: JoinHandle<()>,
    task_upstream: Sender<AudioTask>,
}

type AudioChannels = (
    AudioPlayer,
    Receiver<QueueFramework>,
    Receiver<ProgressUpdate>,
    Receiver<ScrobbleRequest>
);

impl AudioPlayer {
    pub fn new() -> Result<AudioChannels, ResonateError> {
        let (task_upstream, task_downstream) = bounded::<AudioTask>(256);
        let (queue_upstream, queue_downstream) = bounded::<QueueFramework>(256);
        let (progress_upstream, progress_downstream) = bounded::<ProgressUpdate>(256);
        let (scrobble_upstream, scrobble_downstream) = bounded::<ScrobbleRequest>(256);

        let (_stream, handle) = match OutputStream::try_default() {
            Ok(data) => data,
            Err(_) => return Err(ResonateError::AudioStreamError)
        };

        let sink = match Sink::try_new(&handle) {
            Ok(sink) => sink,
            Err(_) => return Err(ResonateError::AudioStreamError)
        };

        let _thread_handle = spawn(
            move || audio_thread(
                sink, task_downstream, queue_upstream, progress_upstream, scrobble_upstream
            )
        );

        Ok((AudioPlayer {
            _stream,
            _handle: handle,
            _thread_handle,
            task_upstream,
        },
            queue_downstream,
            progress_downstream,
            scrobble_downstream
        ))
    }

    pub fn send_task(&self, task: AudioTask) -> Result<(), ()> {
        match self.task_upstream.send_blocking(task) {
            Ok(_) => Ok(()),
            Err(e) => {
                println!("[AUDIO] Error sending task: {e:?}");
                Err(())
            }
        }
    }
}
