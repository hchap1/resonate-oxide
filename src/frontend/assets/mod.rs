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
