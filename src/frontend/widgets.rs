use std::path::Path;

use iced::alignment::{Horizontal, Vertical};
use iced::advanced::svg::Handle;
use iced::widget::{button, progress_bar, slider, text_input, Button, Slider};
use iced::widget::scrollable::Scroller;
use iced::widget::{container, image, scrollable, text, Column, Container, Row, Scrollable, TextInput, svg, ProgressBar};
use iced::{Background, Border, Color, Element, Length, Shadow};

use crate::frontend::message::Message;
use crate::backend::music::{Playlist, Song};
use crate::backend::audio::{AudioTask, ProgressUpdate, QueueFramework};

use super::message::PageType;
use super::search_page::SearchState;

const R: u8 = 78;
const G: u8 = 62;
const B: u8 = 116;

#[allow(dead_code)]
struct ResonateColour;
impl ResonateColour {
    fn new(r: u8, g: u8, b: u8) -> Color { Color::from_rgb8(r, g, b) }
    fn hex(hex: &str) -> Color { Color::parse(hex).unwrap() }

    fn background()     -> Color { Self::hex("#1f2335") }
    fn foreground()     -> Color { Self::hex("#24283b") }
    fn accent()         -> Color { Self::hex("#292e42") }
    fn colour()         -> Color { Self::new(R, G, B) } // { Self::hex("#9d7cd8") }
    fn lighter_colour() -> Color { Self::new(
        (R as f32 * 1.1 as f32).round() as u8,
        (G as f32 * 1.1 as f32).round() as u8,
        (B as f32 * 1.1 as f32).round() as u8,
    )} // { Self::hex("#b992ff") }
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
                border: Border::default().rounded(10),
                scroller: Scroller {
                    color: ResonateColour::colour(),
                    border: Border::default().rounded(10)
                }
            },
            horizontal_rail: scrollable::Rail {
                background: Some(iced::Background::Color(ResonateColour::colour())),
                border: Border::default().rounded(10),
                scroller: Scroller {
                    color: ResonateColour::colour(),
                    border: Border::default().rounded(10)
                }
            },
            gap: Some(iced::Background::Color(ResonateColour::text()))
        }
    }

    fn thumbnail_container() -> container::Style {
        container::Style {
            text_color: None,
            background: None,//Some(iced::Background::Color(ResonateColour::accent())),
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

    fn hightlighted_button_wrapper(status: iced::widget::button::Status) -> button::Style {
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

    fn icon_button_with_background(status: iced::widget::button::Status, on: bool) -> button::Style {
        button::Style {
            background: Some(iced::Background::Color(
                match status {
                    button::Status::Active => 
                        if on { ResonateColour::colour() } else { ResonateColour::background() }
                    button::Status::Disabled => ResonateColour::background(),
                    _ => if on { ResonateColour::lighter_colour() } else { ResonateColour::accent() }
                }
            )),
            text_color: ResonateColour::text(),
            border: Border::default().rounded(10),
            shadow: Shadow::default()
        }
    }

    fn progress_bar() -> progress_bar::Style {
        progress_bar::Style {
            background: Background::Color(ResonateColour::accent()),
            bar: Background::Color(ResonateColour::colour()),
            border: Border::default().rounded(10)
        }
    }
}

pub struct ResonateWidget;
impl ResonateWidget {

    pub fn search_notify<'a>(
        notify: &'a SearchState, default_thumbnail: &'a Path, playlist_id: usize
    ) -> Element<'a, Message> {
        Container::new(
            {
                let mut column = Column::new().spacing(10);

                column = column.push(
                    Row::new().spacing(20)
                        .push(
                            match notify {
                                SearchState::Searching => svg(crate::frontend::assets::yellow_cloud_icon()),
                                SearchState::SearchFailed => svg(crate::frontend::assets::red_cloud_icon()),
                                SearchState::Received(_) => svg(crate::frontend::assets::green_cloud_icon())
                            }.width(32).height(32)
                        ).push(
                            match notify {
                                SearchState::Searching => text("Internet Search Active")
                                    .color(ResonateColour::yellow()),
                                SearchState::SearchFailed => text("Internet Search Failed")
                                    .color(ResonateColour::red()),
                                SearchState::Received(_) => text("Internet Search Successful")
                                    .color(ResonateColour::green())
                            }.size(25).width(Length::Fill)
                    ).push(
                        Self::button_widget(crate::frontend::assets::close())
                            .on_press(Message::RemoveSearchStatus)
                    )
                );

                if let SearchState::Received(songs) = notify {
                    for song in songs.iter() {
                        column = column.push(
                            Self::song(song, default_thumbnail, false, None)
                                .on_press(Message::AddSongToPlaylist(song.clone(), playlist_id))
                        )
                    }
                }

                column
            }
        ).style(|_| ResonateStyle::list_container()).width(Length::Fill).padding(5).into()
    }

    pub fn queue_bar<'a>(
        queue_state: Option<&'a QueueFramework>, default_thumbnail: &'a Path
    ) -> Element<'a, Message> {
        Container::new(
            Self::padded_scrollable({
                let mut column = Column::new().spacing(10);
                let queue_items = match queue_state.as_ref() {
                    Some(queue_state) => {
                        for (idx, song) in queue_state.songs.iter().enumerate() {
                            column = column.push(
                                Self::simple_song(song, default_thumbnail, idx == queue_state.position)
                                    .on_press(Message::AudioTask(AudioTask::Move(idx)))
                            )
                        }
                        column.into()
                    },
                    None => column.into()
                };

                queue_items
            })
        ).into()
    }

    pub fn control_bar<'a>(
        queue_state: Option<&'a QueueFramework>,
        last_page: (PageType, Option<usize>),
        progress_update: Option<ProgressUpdate>,
        volume: f32,
        default_thumbnail: &'a Path,
        default_queue: &'a QueueFramework
    ) -> Element<'a, Message> {

        let (real, queue_state) = match queue_state {
            Some(queue_state) => (true, queue_state),
            None => (false, default_queue)
        };

        Container::new(
            Row::new().spacing(20).push(
                match real {
                    true => match queue_state.songs.get(queue_state.position) {
                        Some(song) => Self::simple_song(song, default_thumbnail, false),
                        None => Self::dummy_song(default_thumbnail)
                    }
                    false => Self::dummy_song(default_thumbnail)
                }.width(Length::FillPortion(1))
            ).push(
                Column::new().width(Length::FillPortion(2)).spacing(10).push(
                    Row::new().spacing(20).align_y(Vertical::Center).width(Length::FillPortion(4))
                    .push(
                        Column::new().push(Row::new().spacing(10).width(Length::Shrink)
                            .push(
                                Self::button_widget(crate::frontend::assets::back())
                                    .on_press(Message::LoadPage(last_page.0 ,last_page.1))
                            ).push(
                                Self::button_widget(crate::frontend::assets::back_skip()).on_press(
                                    Message::AudioTask(AudioTask::SkipBackward)
                                )
                            ).push(
                                Self::button_widget(
                                    match queue_state.playing {
                                        true => crate::frontend::assets::pause(),
                                        false => crate::frontend::assets::play()
                                    }
                                ).on_press(
                                    Message::AudioTask(AudioTask::TogglePlayback)
                                )
                            ).push(
                                Self::button_widget(crate::frontend::assets::forward_skip()).on_press(
                                    Message::AudioTask(AudioTask::SkipForward)
                                )
                            ).push(
                                Self::toggle_button_widget(
                                    crate::frontend::assets::repeat(),
                                    queue_state.repeat
                                ).on_press(
                                    Message::AudioTask(AudioTask::ToggleRepeat)
                                )
                            )
                        ).align_x(Horizontal::Center).width(Length::FillPortion(1))
                    ).push(
                        Slider::new(0f32..=2f32, volume,
                            |value| Message::AudioTask(AudioTask::SetVolume(value))
                        ).style(|_,_| slider::Style {
                            rail: slider::Rail {
                                backgrounds: (
                                    Background::Color(ResonateColour::colour()),
                                    Background::Color(ResonateColour::accent())
                                ),
                                width: 15f32,
                                border: Border::default().rounded(10)
                            },
                            handle: slider::Handle {
                                shape: slider::HandleShape::Circle {
                                    radius: 10f32
                                },
                                background: Background::Color(ResonateColour::colour()),
                                border_width: 0f32,
                                border_color: ResonateColour::colour()
                            }
                        }).step(0.01f32)
                    )
                ).align_x(Horizontal::Center).push(
                    ProgressBar::new(0f32..=1000f32, match progress_update {
                        Some(update) => match update {
                            ProgressUpdate::Nothing => 0f32,
                            ProgressUpdate::Seconds(current, length) => (current as f32 / length as f32) * 1000f32
                        },
                        None => 0f32
                    }).width(Length::FillPortion(1)).style(|_| ResonateStyle::progress_bar())
                            .height(Length::Fixed(45f32))
                )
            )
        ).style(|_| ResonateStyle::list_container()).padding(10).into()
    }

    pub fn button_widget<'a>(icon: Handle) -> Button<'a, Message> {
        button(
            svg(icon).width(32).height(32)
        ).style(|_,state| ResonateStyle::icon_button(state))
    }

    pub fn toggle_button_widget<'a>(icon: Handle, state: bool) -> Button<'a, Message> {
        button(
            svg(icon).width(32).height(32)
        ).style(move |_,status| ResonateStyle::icon_button_with_background(status, state))
    }

    pub fn header<'a>(value: &'a str) -> Element<'a, Message> {
        text(value).size(30).color(ResonateColour::colour()).width(Length::Fill).into()
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

    pub fn playlist<'a>(
        playlist: &'a Playlist,
        hovered: bool,
        input_field: Option<&'a str>,
        idx: usize
    ) -> Button<'a, Message> {
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
            ).push_maybe(
                if hovered { Some(Self::button_widget(crate::frontend::assets::edit_icon())
                    .on_press(Message::StartEditing(idx)).style(|_,state| ResonateStyle::icon_button(state))) }
                else { None }
            ).push_maybe(
                if hovered { Some(Self::button_widget(crate::frontend::assets::play())
                    .on_press(Message::LoadEntirePlaylist(playlist.id, false))) }
                else { None }
            ).push_maybe(
                if hovered { Some(Self::button_widget(crate::frontend::assets::shuffle())
                    .on_press(Message::LoadEntirePlaylist(playlist.id, true))) }
                else { None }
            ).push_maybe(
                if hovered { Some(Self::button_widget(crate::frontend::assets::close())
                    .on_press(Message::DeletePlaylist(playlist.id))) }
                else { None }
            )
        ).padding(5)).style(|_, state| ResonateStyle::button_wrapper(state))
    }

    pub fn dummy_song<'a>(default_thumbnail: &'a Path) -> Button<'a, Message> {
        button(Container::new(Row::new().spacing(20).align_y(Vertical::Center)
            .push(Container::new(image(default_thumbnail)).style(|_|
                ResonateStyle::thumbnail_container()).padding(10)
            ).push(
                Column::new().spacing(10)
                    .push(
                        text("-").width(Length::FillPortion(3)).size(20).color(ResonateColour::text())
                    ).push(
                        text("-").width(Length::FillPortion(2))
                    )
            )
            .push(
                text("-").size(20).color(ResonateColour::text())
            )
        ).padding(5).width(Length::Fill)).style(|_,_|
            button::Style {
                background: Some(Background::Color(ResonateColour::background())),
                text_color: ResonateColour::text(),
                border: Border::default().rounded(10),
                shadow: Shadow::default()
            }
        )
    }

    pub fn simple_song<'a>(song: &'a Song, default_thumbnail: &'a Path, selected: bool) -> Button<'a, Message> {
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
            )
            .push(
                text(song.display_duration()).size(20).color(ResonateColour::text())
            )
        ).padding(5).width(Length::Fill)).style(move |_, state| 
                match selected {
                    true => ResonateStyle::hightlighted_button_wrapper(state),
                    false => ResonateStyle::button_wrapper(state)
                }
            )
    }

    pub fn song<'a>(
        song: &'a Song,
        default_thumbnail: &'a Path,
        is_downloading: bool,
        playlist_id: Option<usize>
    ) -> Button<'a, Message> {

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
                                else { svg(crate::frontend::assets::red_cloud_icon()) }).width(Length::Fixed(32f32))
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
            ).push_maybe(
                match playlist_id {
                    Some(playlist_id) => Some(
                        Self::button_widget(crate::frontend::assets::close())
                            .on_press(Message::RemoveSongFromPlaylist(song.id, playlist_id))
                    ),
                    None => None
                }
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
