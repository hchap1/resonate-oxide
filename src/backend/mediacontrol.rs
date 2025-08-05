use std::io::BufReader;
use std::io::BufRead;
use std::process::Stdio;
use std::process::Command;
use std::thread::spawn;
use std::thread::JoinHandle;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

use async_channel::Sender;
use async_channel::Receiver;
use async_channel::unbounded;

fn create_swift_script_if_missing(path: PathBuf) -> std::io::Result<()> {
    if path.exists() {
        return Ok(());
    }

    let swift_code = r#"import Foundation
import IOKit.hid

let manager = IOHIDManagerCreate(kCFAllocatorDefault, IOOptionBits(kIOHIDOptionsTypeNone))
IOHIDManagerSetDeviceMatching(manager, nil)
var pressedKeys = Set<UInt32>()

let callback: IOHIDValueCallback = { context, result, sender, value in
    let element = IOHIDValueGetElement(value)
    let usagePage = IOHIDElementGetUsagePage(element)
    let usage = IOHIDElementGetUsage(element)
    let pressedInt = IOHIDValueGetIntegerValue(value)
    let pressed = pressedInt != 0

    guard usagePage == 0x0C else { return }

    if pressed {
        if !pressedKeys.contains(usage) {
            pressedKeys.insert(usage)
            switch usage {
            case 0xCD:
                print("0")
            case 179:
                print("1")
            case 180:
                print("2")
            default:
                break
            }
            fflush(stdout)
        }
    } else {
        pressedKeys.remove(usage)
    }
}

IOHIDManagerRegisterInputValueCallback(manager, callback, nil)
IOHIDManagerScheduleWithRunLoop(manager, CFRunLoopGetCurrent(), CFRunLoopMode.defaultMode.rawValue)
IOHIDManagerOpen(manager, IOOptionBits(kIOHIDOptionsTypeNone))
CFRunLoopRun()
"#;

    let mut file = File::create(&path)?;
    file.write_all(swift_code.as_bytes())?;
    file.flush()?;

    Ok(())
}

#[derive(Clone, Debug)]
#[allow(dead_code)]
pub enum MediaPacket {
    TogglePlayback,
    SkipForward,
    SkipBackward,
    CouldNotParse(String),
    Failure
}

pub struct MediaControl {
    _joinhandle: JoinHandle<()>
}

pub fn spawn_mediacontrol_daemon(sender: Sender<MediaPacket>, path: PathBuf) {
    let mut child = match Command::new("swift")
        .arg(
            match path.canonicalize() {
                Ok(path) => path.to_string_lossy().to_string(),
                Err(_) => {
                    let _ = sender.send(MediaPacket::Failure);
                    return;
                }
            }
        )
        .stdout(Stdio::piped())
        .spawn()
    {
        Ok(child) => child,
        Err(_) => {
            let _ = sender.send(MediaPacket::Failure);
            return;
        }
    };

    let stdout = match child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            let _ = sender.send(MediaPacket::Failure);
            return;
        }
    };

    let reader = BufReader::new(stdout);

    for line in reader.lines() {
        let _ = sender.send(match line {
            Ok(l) => match l.as_str() {
                "0" => MediaPacket::TogglePlayback,
                "1" => MediaPacket::SkipForward,
                "2" => MediaPacket::SkipBackward,
                 s  => MediaPacket::CouldNotParse(s.to_string())
            }
            Err(_) => MediaPacket::Failure
        });
    }

    let _ = child.wait();
}

impl MediaControl {
    pub fn new(basedir: PathBuf) -> (Self, Receiver<MediaPacket>) {
        let swift_file = basedir.join("media.swift");
        let _ = create_swift_script_if_missing(swift_file.clone());
        let (sender, receiver) = unbounded();
        (Self {
            _joinhandle: spawn(move || spawn_mediacontrol_daemon(sender, swift_file))
        }, receiver)
    }
}
