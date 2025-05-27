use std::path::Path;

use iced::alignment::Vertical;
use iced::advanced::svg::Handle;
use iced::widget::{button, text_input, Button};
use iced::widget::scrollable::Scroller;
use iced::widget::{container, image, scrollable, text, Column, Container, Row, Scrollable, TextInput, svg};
use iced::{Background, Border, Color, Element, Length, Shadow};

use crate::frontend::message::Message;

use crate::backend::music::{Playlist, Song};

struct ResonateColour;
impl ResonateColour {
    fn new(r: u8, g: u8, b: u8) -> Color { Color::from_rgb8(r, g, b) }
    fn hex(hex: &str) -> Color { Color::parse(hex).unwrap() }

    fn background()     -> Color { Self::hex("#1f2335") }
    fn foreground()     -> Color { Self::hex("#24283b") }
    fn accent()         -> Color { Self::hex("#292e42") }
    fn colour()         -> Color { Self::hex("#9d7cd8") }
    fn lighter_colour() -> Color { Self::hex("#b992ff") }
    fn text()           -> Color { Self::hex("#c0caf5") }
    fn darker()         -> Color { Self::hex("#565f89") }
    fn yellow()         -> Color { Self::hex("#e0cf7e") }
    fn red()            -> Color { Self::hex("#b26b6a") }
    fn green()          -> Color { Self::hex("#9ccc65") }
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
                background: Some(iced::Background::Color(ResonateColour::foreground())),
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

    fn thumbnail_container() -> container::Style {
        container::Style {
            text_color: None,
            background: Some(iced::Background::Color(ResonateColour::accent())),
            border: Border::default().rounded(15),
            shadow: Shadow::default()
        }
    }

    fn search_bar(status: iced::widget::text_input::Status) -> text_input::Style {
        text_input::Style {
            background: iced::Background::Color(
                match status {
                    text_input::Status::Active => ResonateColour::foreground(),
                    text_input::Status::Disabled => ResonateColour::background(),
                    _ => ResonateColour::accent()
                }
            ),
            border: Border::default().rounded(10),
            icon: ResonateColour::colour(),
            placeholder: ResonateColour::darker(),
            value: ResonateColour::text(),
            selection: ResonateColour::colour()
        }
    }

    fn button_wrapper(status: iced::widget::button::Status) -> button::Style {
        button::Style {
            background: Some(iced::Background::Color(
                match status {
                    button::Status::Active => ResonateColour::foreground(),
                    button::Status::Disabled => ResonateColour::background(),
                    _ => ResonateColour::accent()
                }
            )),
            text_color: ResonateColour::text(),
            border: Border::default().rounded(10),
            shadow: Shadow::default()
        }
    }

    fn icon_button(status: iced::widget::button::Status) -> button::Style {
        button::Style {
            background: Some(iced::Background::Color(
                match status {
                    button::Status::Active => ResonateColour::colour(),
                    button::Status::Disabled => ResonateColour::background(),
                    _ => ResonateColour::lighter_colour()
                }
            )),
            text_color: ResonateColour::text(),
            border: Border::default().rounded(10),
            shadow: Shadow::default()
        }
    }
}

pub struct ResonateWidget;
impl ResonateWidget {

    pub fn button_widget<'a>(icon: Handle) -> Button<'a, Message> {
        button(
            svg(icon).width(32).height(32)
        ).style(|_,state| ResonateStyle::icon_button(state))
    }

    pub fn header<'a>(value: &'a str) -> Element<'a, Message> {
        text(value).size(30).color(ResonateColour::colour()).into()
    }

    pub fn inline_button<'a>(text: &'a str) -> Button<'a, Message> {
        button(text).style(|_, _| button::Style {
            background: None,
            text_color: ResonateColour::darker(),
            border: Border::default(),
            shadow: Shadow::default()
        })
    }

    pub fn search_bar<'a>(default: &str, current: &str) -> TextInput<'a, Message> {
        text_input(default, current).style(|_, status| ResonateStyle::search_bar(status))
    }

    pub fn playlist<'a>(playlist: &'a Playlist, input_field: Option<&'a str>, idx: usize) -> Button<'a, Message> {
        button(Container::new(Row::new().spacing(20).align_y(Vertical::Center)
            .push(
                text(&playlist.id).size(15).width(Length::FillPortion(1))
            ).push({
                let element: Element<'_, Message> = match input_field {
                    Some(current_value) => text_input("Name...", current_value)
                        .on_input(Message::TextInput)
                        .on_submit(Message::StopEditing)
                        .style(|_,_| text_input::Style {
                            background: Background::Color(ResonateColour::foreground()),
                            border: Border::default().rounded(10),
                            icon: ResonateColour::text(),
                            placeholder: ResonateColour::darker(),
                            selection: ResonateColour::colour(),
                            value: ResonateColour::text()
                        })
                            .width(Length::FillPortion(15))
                            .size(20)
                        .into(),
                    None => text(&playlist.name).size(20).color(ResonateColour::text()).width(Length::FillPortion(15)).into()
                };
                element}
            ).push(
                button(
                    svg(crate::frontend::assets::edit_icon()).width(32).height(32)
                ).on_press(Message::StartEditing(idx)).style(|_,state| ResonateStyle::icon_button(state))
            )
        ).padding(20)).style(|_, state| ResonateStyle::button_wrapper(state))
    }

    pub fn song<'a>(song: &'a Song, default_thumbnail: &'a Path, is_downloading: bool) -> Button<'a, Message> {

        let is_downloaded = song.music_path.is_some();

        button(Container::new(Row::new().spacing(20).align_y(Vertical::Center)
            .push(
                Container::new(image(match song.thumbnail_path.as_ref() {
                    Some(thumbnail) => thumbnail.as_path(),
                    None => default_thumbnail
                })).style(|_| ResonateStyle::thumbnail_container()).padding(10)
            )
            .push(
                Column::new().spacing(10)
                    .push(
                        text(&song.title).width(Length::FillPortion(3)).size(20).color(ResonateColour::text())
                    ).push(
                        Row::new().spacing(10).align_y(Vertical::Center)
                            .push(
                                (if is_downloading { svg(crate::frontend::assets::downloading_icon()) }
                                else if is_downloaded { svg(crate::frontend::assets::tick_icon()) }
                                else { svg(crate::frontend::assets::cloud_icon()) }).width(Length::Fixed(32f32))
                            )
                            .push(text(&song.artist).width(Length::FillPortion(2)))
                    )
            ).push(
                text(match &song.album {
                    Some(album) => album,
                    None => "No album"
                }).width(Length::FillPortion(3))
            ).push(
                text(song.display_duration()).width(Length::FillPortion(1))
            )
        ).padding(20).width(Length::Fill)).style(|_, state| ResonateStyle::button_wrapper(state))
    }

    pub fn search_result<'a>(song: &'a Song, default_thumbnail: &'a Path) -> Button<'a, Message> {
        button(Container::new(Row::new().spacing(20).align_y(Vertical::Center)
            .push(
                Container::new(image(match song.thumbnail_path.as_ref() {
                    Some(thumbnail) => thumbnail.as_path(),
                    None => default_thumbnail
                })).style(|_| ResonateStyle::thumbnail_container()).padding(10)
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
        ).padding(20).width(Length::Fill)).style(|_, state| ResonateStyle::button_wrapper(state))
    }

    pub fn padded_scrollable<'a>(element: Element<'a, Message>) -> Scrollable<'a, Message> {
            Scrollable::new(
                element
            )
                .style(|_,_| ResonateStyle::scrollable_list())
                .width(Length::Fill)
                .height(Length::Fill)
                .spacing(20)
    }

    pub fn window<'a>(element: Element<'a, Message>) -> Element<'a, Message> {
        Container::new(element).padding(20).width(Length::Fill).height(Length::Fill).style(|_| ResonateStyle::background_wrapper()).into()
    }
}
