use dotenv::dotenv;

use rspotify::client::Spotify;
use rspotify::oauth2::{SpotifyClientCredentials, SpotifyOAuth};
use rspotify::util::get_token;
use scraper::{Html, Selector};
use tokio::fs::create_dir_all;
use tokio::process::Command;
use tokio::time::delay_for;

use std::env;
use std::fmt;
use std::path::PathBuf;
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
    let mut current_track = Track::new();

    loop {
        let track = get_track(&spotify).await?;
        if current_track != track {
            Command::new("clear").spawn()?.await?;
            current_track = match get_track_lyrics(&track).await? {
                Some(lyrics) => track.lyrics(&lyrics),
                None => track,
            };
            println!("{}", current_track);
        }
        delay_for(Duration::from_millis(300)).await
    }
}

async fn get_track_lyrics(t: &Track) -> Result<Option<String>> {
    // let mut text: Option<String>;
    let client = reqwest::Client::new();
    let query = format!("{}+{}+lyrics", t.name, t.artists).replace(" ", "+");

    // try google.com
    let url = format!(
        "https://www.google.com/search?q={}&ie=utf-8&oe=utf-8",
        query
    );
    // println!("{}", url);

    let response = client.get(&url).send().await?.text().await?;
    // println!("{}", response);

    let fragment = Html::parse_fragment(&response);
    let elm = Selector::parse(r#"div[class="BNeawe tAd8D AP7Wnd"]"#).unwrap();

    let text = match fragment.select(&elm).last() {
        Some(elm) => match elm.first_child() {
            Some(node) => Some(node.value().as_text().unwrap().to_string()),
            _ => None,
        },
        _ => None,
    };
    // println!("{:?}", text.clone());

    // TODO: change it
    // try greatlyrics.net
    if text.is_none() {
        // let url = format!(
        //     "http://greatlyrics.net/search.html?q={}&ie=utf-8&oe=utf-8",
        //     query
        // );
        // let response = client.get(&url).send().await?.text().await?;
        // let fragment = Html::parse_fragment(&response);
        // let elm = Selector::parse("a.txt-primary").unwrap();
        // let url = fragment
        //     .select(&elm)
        //     .next()
        //     .unwrap()
        //     .value()
        //     .attr("href")
        //     .unwrap();

        // let response = client.get(url).send().await?.text().await?;
        // let fragment = Html::parse_fragment(&response);
        // let elm = Selector::parse(r#"p[id="lyrics"]"#).unwrap();

        // text = match fragment.select(&elm).next() {
        //     Some(elm) => match elm.first_child() {
        //         Some(node) => {
        //             let s = node.value().as_text().unwrap().to_string();
        //             if s.trim() != "" {
        //                 Some(s.trim().to_string())
        //             } else {
        //                 None
        //             }
        //         }
        //         _ => None,
        //     },
        //     _ => None,
        // };

        // println!("{:?}", text.clone());
    };

    Ok(text)
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

    // path for cache token
    let path = [".config", "spotifyrics"].iter().collect::<PathBuf>();
    let mut home = PathBuf::from(env::var("HOME").unwrap_or_default());
    home.push(path);
    create_dir_all(&home).await?;
    home.push("token_cache.json");

    let mut oauth = SpotifyOAuth::default()
        .client_id(&client_id)
        .client_secret(&client_secret)
        .redirect_uri(&redirect_uri)
        .cache_path(home)
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
