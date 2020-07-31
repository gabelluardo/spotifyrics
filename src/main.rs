use anyhow::{bail, Result};
use dotenv::dotenv;

use rspotify::client::Spotify;
use rspotify::oauth2::{SpotifyClientCredentials, SpotifyOAuth};
use rspotify::util::get_token;

use std::env;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    let client_id = env::var("CLIENT_ID").unwrap_or_default();
    let client_secret = env::var("CLIENT_SECRET").unwrap_or_default();
    let redirect_uri = env::var("REDIRECT_URI").unwrap_or_default();

    let mut oauth = SpotifyOAuth::default()
        .client_id(&client_id)
        .client_secret(&client_secret)
        .redirect_uri(&redirect_uri)
        .scope("user-read-currently-playing")
        .build();

    // loop {
    let track_name = match get_token(&mut oauth).await {
        Some(token_info) => {
            let client_credential = SpotifyClientCredentials::default()
                .token_info(token_info)
                .client_id(&client_id)
                .client_secret(&client_secret)
                .build();

            let spotify = Spotify::default()
                .client_credentials_manager(client_credential)
                .build();

            let result = spotify
                .current_user_playing_track()
                .await
                .expect("get current_playing error");

            match result {
                Some(current_playing) => current_playing.item.unwrap().name,
                _ => bail!("Nothing is playing"),
            }
        }
        _ => bail!("auth failed"),
    };

    println!("{}", track_name);
    //     std::thread::sleep(std::time::Duration::from_millis(100000))
    // }

    Ok(())
}
