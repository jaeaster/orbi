use std::{io::Cursor, str::FromStr, sync::Arc};

use chrono::{DateTime, Utc};
use dotenv::dotenv;
use eyre::Result;

use image::{DynamicImage, ImageFormat};
use teloxide::{
    prelude::*,
    types::{InputFile, MediaKind, MediaText, MessageKind},
};

static LAYERS_DIR: &str = "orbitalz-layers";
static LAYERS_ORDER: [&str; 6] = ["Background", "Orbital", "Eyes", "Nose", "Mouth", "Hat"];

#[tokio::main]
async fn main() -> Result<()> {
    run().await?;
    Ok(())
}

async fn run() -> Result<()> {
    dotenv().ok();
    pretty_env_logger::init();

    log::debug!("Parsing layer groups");
    let mut layer_groups = nftgen::layer::get_layer_groups(LAYERS_DIR, &LAYERS_ORDER)?;
    log::debug!(
        "Sorting layer groups according to order: {}",
        LAYERS_ORDER.join(", ")
    );
    layer_groups.sort_by(|a, b| a.partial_cmp(b).unwrap());

    log::info!("Starting Orbi...");
    let bot = Bot::from_env().auto_send();

    let layer_groups = Arc::new(layer_groups);
    teloxide::repl(bot, move |message: Message, bot: AutoSend<Bot>| {
        let layer_groups = layer_groups.clone();
        async move {
            let layer_groups = layer_groups.clone();
            let timestamp_result: Result<DateTime<Utc>, chrono::ParseError> =
                DateTime::from_str("2022-06-13T16:56:51Z");

            if let Ok(last_message_timestamp) = timestamp_result {
                if message.date < last_message_timestamp {
                    return Ok(());
                }
            }

            log::debug!("{:#?}", message);
            match message.kind {
                MessageKind::Common(msg) => {
                    if let Some(from) = msg.from {
                        log::info!("Received message from: {:#?}", from.first_name);
                    } else {
                        log::info!("Received message from anon");
                    }
                    match msg.media_kind {
                        MediaKind::Text(media_text) => {
                            log::info!("{}", media_text.text);
                            if bot_mentioned(&media_text) {
                                let orbital_image = gen_orbital(&layer_groups);
                                log::info!("Sending orbitalz");
                                bot.send_photo(message.chat.id, orbital_image).await?;
                            }
                        }
                        _ => (),
                    }
                }
                _ => (),
            }
            respond(())
        }
    })
    .await;

    Ok(())
}

fn bot_mentioned(media_text: &MediaText) -> bool {
    media_text
        .text
        .to_lowercase()
        .split(" ")
        .filter(|&word| word == "orbi" || word == "@orbitalz_bot")
        .collect::<Vec<&str>>()
        .len()
        > 0
}

fn gen_orbital(layer_groups: &[nftgen::layer::LayerGroup]) -> InputFile {
    log::info!("Generating Orbital");
    let (nft, _) = nftgen::image_builder::ImageBuilder::build(layer_groups);
    let mut png_buf = Cursor::new(vec![]);
    nft.write_to(&mut png_buf, ImageFormat::Png).unwrap();
    InputFile::memory(png_buf.into_inner())
}
