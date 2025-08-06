#![allow(dead_code)]
use std::default::Default;
use std::path::Path;
use std::fs::read_to_string;

use iced::Color;

use crate::frontend::widgets::ResonateColour;

#[allow(clippy::enum_variant_names)]
#[derive(Clone, Debug)]
pub enum Secret {
    SpotifyID(String),
    SpotifySecret(String),
    FMKey(String),
    FMSecret(String),
    FMSession(String),
}


pub struct Settings {
    pub colour: Color,
    pub max_download_concurrency: usize
}

enum Setting {
    Colour,
    MaxDownloadConcurrency
}

impl Setting {
    fn from_string(string: &str) -> Option<Setting> {
        match string.to_lowercase().as_str() {
            "colour" => Some(Setting::Colour),
            "max_download_concurrency" => Some(Setting::MaxDownloadConcurrency),
            _ => None
        }
    }
}

struct ConfigLine {
    target_setting: Setting,
    value: String
}

impl Settings {
    pub fn load(directory: &Path) -> Settings {
        let config_path = directory.join(".conf");
        if config_path.exists() {
            let lines: Vec<ConfigLine> = match read_to_string(config_path) {
                Ok(lines) => lines
                    .lines()
                    .filter_map(
                        |x| {
                            let mut args = x
                                .split("=")
                                .map(|b| b.to_string().trim().to_string())
                                .collect::<Vec<String>>();

                            if args.len() == 2 {
                                Setting::from_string(&args[0]).map(
                                    |setting| ConfigLine { target_setting: setting, value: args.pop().unwrap() }
                                )
                                
                            } else {
                                None
                            }
                        }
                    )
                    .collect(),
                Err(_) => return Settings::default()
            };

            let mut settings = Settings::default();

            lines.into_iter().for_each(
                |line| match line.target_setting {
                    Setting::Colour => settings.colour = ResonateColour::hex(&line.value),
                    Setting::MaxDownloadConcurrency => if let Ok(value) = line.value.parse::<usize>() {
                        settings.max_download_concurrency = value
                    }
                }
            );

            settings
        } else {
            Settings::default()
        }
    }
}

impl Default for Settings {
    fn default() -> Settings {
        Settings {
            colour: Color::from_rgb8(255, 0, 0),
            max_download_concurrency: 4
        }
    }
}
