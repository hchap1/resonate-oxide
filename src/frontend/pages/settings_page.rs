use std::collections::HashSet;

use iced::widget::Column;
use iced::widget::Row;
use iced::Task;

use crate::frontend::application::Page;
use crate::frontend::message::Message;
use crate::frontend::message::PageType;
use crate::frontend::widgets::ResonateWidget;

use crate::backend::util::AM;
use crate::backend::database::Database;
use crate::backend::music::Song;
use crate::backend::settings::Secret;

pub struct SettingsPage {
    database: AM<Database>,

    spotify_id: Option<String>,
    spotify_secret: Option<String>,

    fm_key: Option<String>,
    fm_secret: Option<String>,
    fm_session: Option<String>
}

impl SettingsPage {
    pub fn new(database: AM<Database>) -> Self {
        let mut s = Self {
            database,
            spotify_id: None,
            spotify_secret: None,
            fm_key: None,
            fm_secret: None,
            fm_session: None
        };
        s.reload_settings();
        s
    }

    fn reload_settings(&mut self) {
        self.spotify_id = self.database.lock().unwrap().get_secret("SPOTIFY_ID");
        self.spotify_secret = self.database.lock().unwrap().get_secret("SPOTIFY_SECRET");
        self.fm_key = self.database.lock().unwrap().get_secret("FM_KEY");
        self.fm_secret = self.database.lock().unwrap().get_secret("FM_SECRET");
        self.fm_session = self.database.lock().unwrap().get_secret("FM_SESSION");
    }
}

impl Page for SettingsPage {
    fn view(&self, _: &HashSet<String>, _: &HashSet<Song>) -> Column<Message> {
        Column::new().push(
            Row::new().spacing(10).push(
                Column::new().spacing(20)
                    .push(
                        ResonateWidget::search_bar(
                            "SPOTIFY ID", match self.spotify_id.as_ref() {
                                Some(val) => val.as_str(),
                                None => ""
                            }
                        )
                            .on_input(|x| Message::ChangeSecret(Secret::SpotifyID(x)))
                            .on_paste(|x| Message::ChangeSecret(Secret::SpotifyID(x)))
                    ).push(
                        ResonateWidget::search_bar(
                            "SPOTIFY SECRET", match self.spotify_secret.as_ref() {
                                Some(val) => val.as_str(),
                                None => ""
                            }
                        )
                            .on_input(|x| Message::ChangeSecret(Secret::SpotifySecret(x)))
                            .on_paste(|x| Message::ChangeSecret(Secret::SpotifySecret(x)))
                    ).push(
                        ResonateWidget::search_bar(
                            "FM KEY", match self.fm_key.as_ref() {
                                Some(val) => val.as_str(),
                                None => ""
                            }
                        )
                            .on_input(|x| Message::ChangeSecret(Secret::FMKey(x)))
                            .on_paste(|x| Message::ChangeSecret(Secret::FMKey(x)))
                    ).push(
                        ResonateWidget::search_bar(
                            "FM SECRET", match self.fm_secret.as_ref() {
                                Some(val) => val.as_str(),
                                None => ""
                            }
                        )
                            .on_input(|x| Message::ChangeSecret(Secret::FMSecret(x)))
                            .on_paste(|x| Message::ChangeSecret(Secret::FMSecret(x)))
                    ).push(
                        ResonateWidget::search_bar(
                            "FM SESSION", match self.fm_session.as_ref() {
                                Some(val) => val.as_str(),
                                None => ""
                            }
                        )
                            .on_input(|x| Message::ChangeSecret(Secret::FMSession(x)))
                            .on_paste(|x| Message::ChangeSecret(Secret::FMSession(x)))
                    )
            ).push(Column::new().spacing(20)
                .push(ResonateWidget::button_widget(crate::frontend::assets::save_icon())
                    .on_press_maybe(self.spotify_id.as_ref().map(|x| Message::SaveSecret(
                        Secret::SpotifyID(x.clone())
                ))))
                .push(ResonateWidget::button_widget(crate::frontend::assets::save_icon())
                    .on_press_maybe(self.spotify_secret.as_ref().map(|x| Message::SaveSecret(
                        Secret::SpotifySecret(x.clone())
                ))))
                .push(ResonateWidget::button_widget(crate::frontend::assets::save_icon())
                    .on_press_maybe(self.fm_key.as_ref().map(|x| Message::SaveSecret(
                        Secret::FMKey(x.clone())
                ))))
                .push(ResonateWidget::button_widget(crate::frontend::assets::save_icon())
                    .on_press_maybe(self.fm_secret.as_ref().map(|x| Message::SaveSecret(
                        Secret::FMSecret(x.clone())
                ))))
                .push(ResonateWidget::button_widget(crate::frontend::assets::save_icon())
                    .on_press_maybe(self.fm_session.as_ref().map(|x| Message::SaveSecret(
                        Secret::FMSession(x.clone())
                ))))
            )
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::ChangeSecret(secret) => {
                match secret {
                    Secret::FMKey(new_val) => self.fm_key = Some(new_val),
                    Secret::FMSecret(new_val) => self.fm_secret = Some(new_val),
                    Secret::FMSession(new_val) => self.fm_session = Some(new_val),
                    Secret::SpotifyID(new_val) => self.spotify_id = Some(new_val),
                    Secret::SpotifySecret(new_val) => self.spotify_secret = Some(new_val),
                }
            }

            _ => {}
        }
        Task::none()
    }

    fn back(&self, previous_page: (PageType, Option<usize>)) -> (PageType, Option<usize>) {
        previous_page
    }
}
