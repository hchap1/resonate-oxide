macro_rules! svg {
    ($n:ident, $f:expr) => {
        pub fn $n() -> iced::advanced::svg::Handle {
            return iced::advanced::svg::Handle::from_memory(include_bytes!($f).as_slice());
        }
    }
}

svg!(edit_icon, "icons/edit.svg");
svg!(home_icon, "icons/home.svg");
svg!(tick_icon, "icons/check.svg");
svg!(downloading_icon, "icons/downloading.svg");
svg!(cloud_icon, "icons/cloud.svg");

svg!(back_skip, "icons/back_skip.svg");
svg!(forward_skip, "icons/forward_skip.svg");

svg!(play, "icons/play.svg");
svg!(pause, "icons/pause.svg");
