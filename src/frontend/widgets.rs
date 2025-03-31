use iced::widget::{Column, Row, Container, text};
use iced::Element;

use crate::frontend::message::Message;

use crate::backend::music::Song;

pub struct ResonateWidget;
impl ResonateWidget {
    pub fn search_result<'a>(song: &'a Song) -> Element<'a, Message> {
        Container::new(Row::new()
            .push(
                Column::new()
                    .push(
                        text(&song.title)
                    ).push(
                        text(&song.artist)
                    )
            ).push(
                text(match &song.album {
                    Some(album) => album,
                    None => "No album"
                })
            ).push(
                text(song.display_duration())
            )
        ).into()
    }
}
