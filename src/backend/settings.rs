use std::default::Default
use std::path::Path;
use std::fs::read_to_string;

use iced::Color;

use crate::frontend::widgets::ResonateColour;

pub struct Settings {
    colour: Color,
    max_download_concurrency: usize
}

enum Setting {
    Colour,
    MaxDownloadConcurrency
}

impl Setting {
    fn from_string(string: &String) -> Option<Setting> {
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
                    .into_iter()
                    .filter_map(
                        |x| {
                            let mut args = x
                                .to_string()
                                .split("=")
                                .into_iter()
                                .map(|b| b.to_string().trim().to_string())
                                .collect::<Vec<String>>();

                            if args.len() == 2 {
                                match Setting::from_string(&args[0]) {
                                    Some(setting) => Some(ConfigLine {
                                        target_setting: setting,
                                        value: args.pop().unwrap()
                                    }),
                                    None => None
                                }
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
                    Setting::MaxDownloadConcurrency => match line.value.parse::<usize>() {
                        Ok(value) => settings.max_download_concurrency = value,
                        Err(_) => {}
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
            colour: ResonateColour::colour(),
            max_download_concurrency: 4
        }
    }
}
