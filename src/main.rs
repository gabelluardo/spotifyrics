use dotenv::dotenv;

use rspotify::client::Spotify;
use rspotify::oauth2::{SpotifyClientCredentials, SpotifyOAuth};
use rspotify::util::get_token;
use scraper::{Html, Selector};
use tokio::process::Command;
use tokio::time::delay_for;

use std::env;
use std::fmt;
use std::time::Duration;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

#[derive(Clone)]
struct Track {
    artists: String,
    lyrics: String,
    name: String,
}

impl fmt::Display for Track {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let separator = "-".repeat(self.name.len() + self.artists.len() + 5);

        write!(
            f,
            "+{}+\n| {} - {} |\n+{}+\n\n{}",
            separator, self.name, self.artists, separator, self.lyrics
        )
    }
}

impl Default for Track {
    fn default() -> Self {
        Self {
            name: Default::default(),
            artists: Default::default(),
            lyrics: String::from("No lyrics found"),
        }
    }
}

impl PartialEq for Track {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name && self.artists == other.artists
    }
}

impl Track {
    fn new() -> Self {
        Self::default()
    }

    fn name(self, name: &str) -> Self {
        Self {
            name: name.to_string(),
            ..self
        }
    }

    fn artists(self, artists: &str) -> Self {
        Self {
            artists: artists.to_string(),
            ..self
        }
    }

    fn lyrics(self, lyrics: &str) -> Self {
        Self {
            lyrics: lyrics.to_string(),
            ..self
        }
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let spotify = init_spotify().await?;
    let mut track = Track::new();

    loop {
        let current_track = get_track(&spotify).await?;
        if current_track != track {
            track = current_track;

            Command::new("clear").spawn()?.await?;

            let t = track.clone().lyrics(&get_track_lyrics(&track).await?);
            println!("{}", t);
        }
        delay_for(Duration::from_millis(300)).await
    }
}

async fn get_track_lyrics(t: &Track) -> Result<String> {
    let query = format!("{}+{}+lyrics", t.name, t.artists).replace(" ", "+");

    // try google.com
    let url = format!(
        "https://www.google.com/search?q={}&ie=utf-8&oe=utf-8",
        query
    );
    // let url = "https://www.google.com/search?q=Someone+You+Loved+Lewis+Capaldi+lyrics&oe=utf-8";

    let mut _text: Option<String>;

    // println!("{}", url);

    let response = reqwest::get(&url).await?.text().await?;
    // println!("{}", response);

    let fragment = Html::parse_fragment(&response);
    let elm = Selector::parse(r#"div[class="BNeawe tAd8D AP7Wnd"]"#).unwrap();

    _text = match fragment.select(&elm).skip(3).next() {
        Some(elm) => match elm.first_child() {
            Some(node) => Some(node.value().as_text().unwrap().to_string()),
            _ => None,
        },
        _ => None,
    };

    // try another site
    if _text.is_none() {
        todo!()
    };
    // println!("{}", _text.clone().unwrap());

    Ok(_text.unwrap())
}

async fn get_track(spotify: &Spotify) -> Result<Track> {
    let track = spotify
        .current_user_playing_track()
        .await
        .expect("get current_playing error");
    let item = track.ok_or("Nothing is playing")?.item.unwrap();

    let name = item.name;
    let artists = item
        .artists
        .into_iter()
        .map(|a| a.name.trim().to_string())
        .collect::<Vec<_>>()
        .join(", ");

    Ok(Track::new().name(&name).artists(&artists))
}

async fn init_spotify() -> Result<Spotify> {
    let client_id = env::var("CLIENT_ID").unwrap_or_default();
    let client_secret = env::var("CLIENT_SECRET").unwrap_or_default();
    let redirect_uri = env::var("REDIRECT_URI").unwrap_or_default();

    let mut oauth = SpotifyOAuth::default()
        .client_id(&client_id)
        .client_secret(&client_secret)
        .redirect_uri(&redirect_uri)
        .scope("user-read-currently-playing")
        .build();
    let token_info = get_token(&mut oauth).await.ok_or("auth failed")?;

    let client_credential = SpotifyClientCredentials::default()
        .token_info(token_info)
        .client_id(&client_id)
        .client_secret(&client_secret)
        .build();

    Ok(Spotify::default()
        .client_credentials_manager(client_credential)
        .build())
}
