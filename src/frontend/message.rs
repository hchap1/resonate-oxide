#[derive(Clone, Debug)]
pub enum Message {
    LoadPage(PageType),
    TextInput(String),
    SubmitSearch
}

#[derive(Clone, Debug)]
pub enum PageType {
    SearchSongs
}
