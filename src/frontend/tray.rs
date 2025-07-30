use tray_icon::{
    menu::{Menu, MenuEvent, MenuItem}, TrayIcon, TrayIconBuilder
};

use crossbeam_channel::unbounded;
use crossbeam_channel::Receiver;

use std::thread;
use image::GenericImageView;

const ICON_BYTES: &[u8] = include_bytes!("assets/icons/icon.png");

#[derive(Debug)]
pub enum TrayMessage {
    OpenMain,
    Quit
}

pub struct SimpleTray {
    _tray_icon: TrayIcon,
    _event_thread: thread::JoinHandle<()>,
    out: Option<Receiver<TrayMessage>>
}

impl SimpleTray {

    pub fn take_receiver(&mut self) -> Option<Receiver<TrayMessage>> {
        self.out.take()
    }

    pub fn new() -> Self {
    let img = image::load_from_memory(ICON_BYTES).expect("Failed to load embedded image");

    let rgba = img.to_rgba8();
    let (width, height) = img.dimensions();

    let icon = tray_icon::Icon::from_rgba(rgba.into_vec(), width, height)
        .expect("Failed to create tray icon");

    let menu = Menu::new();

    let open_item = MenuItem::new("Open", true, None);
    menu.append(&open_item).expect("Failed to append menu item");

    let close_item = MenuItem::new("Quit", true, None);
    menu.append(&close_item).expect("Failed to append menu item");

    let tray_icon = TrayIconBuilder::new()
        .with_icon(icon)
        .with_menu(Box::new(menu))
        .build()
        .expect("Failed to build tray icon");

    let open_item_id = open_item.id().clone();
    let close_item_id = close_item.id().clone();

    let (sender, receiver) = unbounded();

    let event_thread = thread::spawn(move || {
        let receiver = MenuEvent::receiver();
            loop {
                if let Ok(evt) = receiver.recv() {
                    println!("RECEIVED TRAY EVENT");
                    if let Some(message) = if evt.id == open_item_id {
                        println!("TRAY EVEN WAS OPENMAIN");
                        Some(TrayMessage::OpenMain)
                    } else if evt.id == close_item_id {
                        Some(TrayMessage::Quit)
                    } else {
                        None
                    } {
                        let _ = sender.send(message);
                        println!("TRAY MESSAGE SENT");
                    }
                }
            }
        });

        Self {
            _tray_icon: tray_icon,
            _event_thread: event_thread,
            out: Some(receiver)
        }
    }
}
