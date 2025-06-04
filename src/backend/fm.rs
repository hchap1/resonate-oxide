pub struct FMAuth {
    pub key: Option<String>,
    pub secret: Option<String>,
    pub username: Option<String>,
    pub password: Option<String>
}

impl FMAuth {
    pub fn new(
        key: Option<String>, secret: Option<String>, username: Option<String>, password: Option<String>
    ) -> FMAuth {
        FMAuth { key, secret, username, password }
    }
}
