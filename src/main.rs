use anyhow::Context;
use serde::Deserialize;

use std::io::prelude::*;

//
// Constants
//

const IOTD_FEED_URL: &str = "https://www.nasa.gov/rss/dyn/lg_image_of_the_day.rss";

//
// XML Structure
//

#[derive(Deserialize)]
struct Rss {
    channel: Channel,
}

#[derive(Deserialize)]
struct Channel {
    #[serde(rename = "item", default)]
    items: Vec<Item>,
}

#[derive(Deserialize)]
struct Item {
    enclosure: Enclosure,
}

#[derive(Deserialize)]
struct Enclosure {
    url: String,
}

#[tokio::main]
async fn main() {
    loop {
        while let Err(e) = update_wallpaper().await {
            eprintln!("{:?}", e);
            std::thread::sleep(std::time::Duration::from_secs(3));
        }

        // Sleep 12 hours
        std::thread::sleep(std::time::Duration::from_secs(60 * 60 * 12));
    }
}

async fn update_wallpaper() -> anyhow::Result<()> {
    // Create http client
    let client = reqwest::Client::builder()
        .user_agent("NASA IOTD Wallpaper")
        .build()
        .expect("Could not build http client");

    println!("Downloading RSS Feed: {}", IOTD_FEED_URL);
    // Download the latest RSS feed
    let xml_data = client
        .get(IOTD_FEED_URL)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await
        .context("Could not download RSS feed")?;

    println!("Parsing XML");
    // Deserialize the XML file
    let feed: Rss = quick_xml::de::from_str(&xml_data)?;

    println!("Getting latest image of the day");
    // If there is a latest item
    if let Some(item) = feed.channel.items.get(0) {
        let mut image_url = url::Url::parse(&item.enclosure.url)?;
        // Work around this issue with http: https://github.com/seanmonstar/reqwest/issues/992
        image_url.set_scheme("https").expect("Invalid scheme");

        println!("Downloading image: {}", image_url);

        // Download the image
        let mut image_data = client
            .get(image_url)
            .send()
            .await?
            .error_for_status()?
            .bytes()
            .await
            .context("Could not download image")?;

        let image_path = std::env::temp_dir().join("nasa-iotd.jpg");

        println!("Writing file to disk");
        // Create a temporary file to store the image in
        let mut tempfile = std::fs::OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(&image_path)?;

        // Write the image to the file
        tempfile.write_all(&mut image_data)?;

        println!("Setting desktop wallpaper");
        // Set the desktop wallpaper
        wallpaper::set_from_path(&image_path.to_string_lossy())
            .unwrap_or_else(|e| eprintln!("{:?}", e));

    // Bail out if there is no image of the day
    } else {
        anyhow::bail!("There is not image of the day!");
    }

    Ok(())
}
