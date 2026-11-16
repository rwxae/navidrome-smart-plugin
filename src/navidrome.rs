use rand::distr::{Alphanumeric, SampleString};

pub struct Client {
    api_base: String,
    client: String,
    version: String,
    user: String,
    token: String,
    salt: String,
}

impl Client {
    pub fn new(url: &str, client: &str, version: &str, user: &str, password: &str) -> Self {
        let salt = Alphanumeric.sample_string(&mut rand::rng(), 12);
        let token = format!("{:x}", md5::compute(format!("{password}{salt}")));
        Self {
            api_base: url.into(),
            client: client.into(),
            version: version.into(),
            user: user.into(),
            token,
            salt,
        }
    }

    fn common_params(&self) -> [(&str, &str); 6] {
        [
            ("u", &self.user),
            ("t", &self.token),
            ("s", &self.salt),
            ("v", &self.version),
            ("c", &self.client),
            ("f", "json"),
        ]
    }

    pub fn create_playlist<T, K>(&self, name: &str, songs: T) -> attohttpc::Result<bool>
    where
        T: IntoIterator<Item = K>,
        K: ToString,
    {
        attohttpc::post(format!("{}/rest/createPlaylist", self.api_base))
            .params(self.common_params())
            .param("name", name)
            .param("playlistId", "5rv8I3rbnM8OKRRcHAqnWs") // TODO: use from store
            .params(songs.into_iter().map(|id| ("songId", id)))
            .send()?;

        Ok(true)
    }
}
