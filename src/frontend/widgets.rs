use std::path::Path;

use iced::widget::scrollable::Scroller;
use iced::widget::{container, image, scrollable, text, Column, Container, Row, Scrollable};
use iced::{Border, Color, Element, Length, Shadow};

use crate::frontend::message::Message;
use crate::backend::music::Song;

struct ResonateColour;
impl ResonateColour {
    fn new(r: u8, g: u8, b: u8) -> Color { Color::from_rgb8(r, g, b) }
    fn hex(hex: &str) -> Color { Color::parse(hex).unwrap() }

    fn background() -> Color { Self::hex("#1f2335") }
    fn foreground() -> Color { Self::hex("#24283b") }
    fn accent()     -> Color { Self::hex("#292e42") }
    fn colour()     -> Color { Self::hex("#9d7cd8") }
    fn text()       -> Color { Self::hex("#c0caf5") }
}

struct ResonateStyle;
impl ResonateStyle {
    fn background_wrapper() -> container::Style {
        container::Style {
            background: Some(iced::Background::Color(ResonateColour::background())),
            border: Border::default(),
            shadow: Shadow::default(),
            text_color: Some(ResonateColour::text())
        }
    }

    fn list_container() -> container::Style {
        container::Style {
            background: Some(iced::Background::Color(ResonateColour::foreground())),
            border: Border::default().rounded(10),
            shadow: Shadow::default(),
            text_color: Some(ResonateColour::colour())
        }
    }

    fn scrollable_list() -> scrollable::Style {
        scrollable::Style {
            container: ResonateStyle::background_wrapper(),
            vertical_rail: scrollable::Rail {
                background: Some(iced::Background::Color(ResonateColour::colour())),
                border: Border::default(),
                scroller: Scroller {
                    color: ResonateColour::colour(),
                    border: Border::default()
                }
            },
            horizontal_rail: scrollable::Rail {
                background: Some(iced::Background::Color(ResonateColour::colour())),
                border: Border::default(),
                scroller: Scroller {
                    color: ResonateColour::colour(),
                    border: Border::default()
                }
            },
            gap: Some(iced::Background::Color(ResonateColour::text()))
        }
    }
}

pub struct ResonateWidget;
impl ResonateWidget {
    pub fn search_result<'a>(song: &'a Song, default_thumbnail: &'a Path) -> Element<'a, Message> {
        Container::new(Row::new()
            .push(
                image(match song.thumbnail_path.as_ref() {
                    Some(thumbnail) => thumbnail.as_path(),
                    None => default_thumbnail
                })
            )
            .push(
                Column::new().spacing(10)
                    .push(
                        text(&song.title).width(Length::FillPortion(3)).size(20).color(ResonateColour::text())
                    ).push(
                        text(&song.artist).width(Length::FillPortion(2))
                    )
            ).push(
                text(match &song.album {
                    Some(album) => album,
                    None => "No album"
                }).width(Length::FillPortion(3))
            ).push(
                text(song.display_duration()).width(Length::FillPortion(1))
            )
        ).padding(20).width(Length::Fill).style(|_| ResonateStyle::list_container()).into()
    }

    pub fn padded_scrollable<'a>(element: Element<'a, Message>) -> Element<'a, Message> {
            Scrollable::new(
                element
            )
                .style(|_,_| ResonateStyle::scrollable_list())
                .width(Length::Fill)
                .height(Length::Fill)
                .spacing(20)
            .into()
    }

    pub fn window<'a>(element: Element<'a, Message>) -> Element<'a, Message> {
        Container::new(element).padding(20).width(Length::Fill).height(Length::Fill).style(|_| ResonateStyle::background_wrapper()).into()
    }
}
