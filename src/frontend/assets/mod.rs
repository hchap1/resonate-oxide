macro_rules! svg {
    ($n:ident, $f:expr) => {
        pub fn $n() -> iced::advanced::svg::Handle {
            return iced::advanced::svg::Handle::from_memory(include_bytes!($f).as_slice());
        }
    }
}

svg!(edit_icon, "icons/edit.svg");
svg!(tick_icon, "icons/check.svg");
svg!(downloading_icon, "icons/downloading.svg");

svg!(red_cloud_icon, "icons/red_cloud.svg");
svg!(yellow_cloud_icon, "icons/yellow_cloud.svg");
svg!(green_cloud_icon, "icons/green_cloud.svg");

svg!(back_skip, "icons/back_skip.svg");
svg!(forward_skip, "icons/forward_skip.svg");

svg!(play, "icons/play.svg");
svg!(pause, "icons/pause.svg");
svg!(shuffle, "icons/shuffle.svg");

svg!(repeat, "icons/repeat_gray.svg");
svg!(refresh, "icons/refresh.svg");

svg!(back, "icons/back.svg");
svg!(close, "icons/delete.svg");
svg!(save_icon, "icons/save.svg");
