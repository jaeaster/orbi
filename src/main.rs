use std::{io::Cursor, sync::Arc};

use dotenv::dotenv;
use eyre::Result;

use image::ImageFormat;
use nftgen::layer::LayerGroup;
use teloxide::{
    prelude::*,
    types::{InputFile, MediaKind, MediaText, MessageKind},
    utils::command::BotCommands,
};

mod s3;
mod webhook;
use crate::s3::download_layers_s3;
use crate::webhook::webhook;

static LAYERS_DIR: &str = "orbitalz-layers";
static LAYERS_ORDER: [&str; 6] = ["Background", "Orbital", "Eyes", "Nose", "Mouth", "Hat"];

#[tokio::main]
async fn main() -> Result<()> {
    let (layer_groups, bot) = init().await?;
    // tokio::time::sleep(tokio::time::Duration::new(1000, 0)).await;
    run(layer_groups, bot).await?;
    Ok(())
}

#[derive(BotCommands, Clone)]
#[command(rename = "lowercase", description = "These commands are supported:")]
enum Command {
    #[command(description = "display this text.")]
    Help,
    #[command(description = "Generate a new Orbital")]
    Create,
    // #[command(description = "handle a username and an age.", parse_with = "split")]
    // UsernameAndAge { username: String, age: u8 },
}

async fn init() -> Result<(Vec<LayerGroup>, AutoSend<Bot>)> {
    dotenv().ok();
    pretty_env_logger::init();

    if !std::path::Path::new(&LAYERS_DIR).exists() {
        log::info!("Downloading layers from S3");
        download_layers_s3().await?;
    }

    log::debug!("Parsing layer groups");
    let mut layer_groups = nftgen::layer::get_layer_groups(LAYERS_DIR, &LAYERS_ORDER)?;
    log::debug!(
        "Sorting layer groups according to order: {}",
        LAYERS_ORDER.join(", ")
    );
    layer_groups.sort_by(|a, b| a.partial_cmp(b).unwrap());

    log::info!("Starting Orbi...");
    let bot = Bot::from_env().auto_send();

    Ok((layer_groups, bot))
}

async fn run(layer_groups: Vec<LayerGroup>, bot: AutoSend<Bot>) -> Result<()> {
    let layer_groups = Arc::new(layer_groups);
    let closure = move |bot: AutoSend<Bot>, message: Message, command: Command| {
        let layer_groups = layer_groups.clone();
        async move {
            let layer_groups = layer_groups.clone();
            log_message(&message);
            match command {
                Command::Help => {
                    bot.send_message(message.chat.id, Command::descriptions().to_string())
                        .await?;
                }
                Command::Create => {
                    let orbital_image = gen_orbital(&layer_groups);
                    log::info!("Sending orbitalz");
                    bot.send_photo(message.chat.id, orbital_image.unwrap())
                        .await?;
                }
            }
            respond(())
        }
    };

    if std::env::var("LOCALHOST").is_ok() {
        log::info!("Running in localhost mode");
        teloxide::commands_repl(bot.clone(), closure, Command::ty()).await;
    } else {
        teloxide::commands_repl_with_listener(
            bot.clone(),
            closure,
            webhook(bot).await,
            Command::ty(),
        )
        .await;
    }
    Ok(())
}

fn log_message(message: &Message) {
    log::debug!("{:#?}", message);
    match &message.kind {
        MessageKind::Common(msg) => {
            if let Some(from) = &msg.from {
                log::info!("Received message from: {:#?}", from.first_name);
            } else {
                log::info!("Received message from anon");
            }
            match &msg.media_kind {
                MediaKind::Text(media_text) => {
                    log::info!("{}", media_text.text);
                }
                _ => (),
            }
        }
        _ => (),
    }
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

fn gen_orbital(layer_groups: &[nftgen::layer::LayerGroup]) -> Result<InputFile> {
    log::info!("Generating Orbital");
    let (nft, _) = nftgen::image_builder::ImageBuilder::build(layer_groups)?;
    let mut png_buf = Cursor::new(vec![]);
    nft.write_to(&mut png_buf, ImageFormat::Png).unwrap();
    Ok(InputFile::memory(png_buf.into_inner()))
}
