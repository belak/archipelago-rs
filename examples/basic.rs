use anyhow::Context;
use archipelago::client::AnonymousClient;
use futures::StreamExt;

#[tokio::main(flavor = "current_thread")]
async fn main() -> anyhow::Result<()> {
    let mut client = AnonymousClient::new(
        std::env::var("ARCHIPELAGO_HOST").context("missing ARCHIPELAGO_HOST")?,
    )
    .await?;

    println!("Connected!");

    println!("Starting Handshake");

    let mut client = client
        .connect(
            std::env::var("ARCHIPELAGO_PASS").ok(),
            std::env::var("ARCHIPELAGO_GAME").unwrap_or_default(),
            std::env::var("ARCHIPELAGO_NAME").context("missing ARCHIPELAGO_GAME")?,
            vec!["AP", "TextClient"],
            archipelago::protocol::ItemsHandlingFlags::CAN_RECEIVE_ITEMS
                | archipelago::protocol::ItemsHandlingFlags::HAS_LOCAL_ITEMS
                | archipelago::protocol::ItemsHandlingFlags::REQUEST_STARTING_INVENTORY,
        )
        .await?;

    println!("Successful Handshake");

    while let Some(message) = client.next().await.transpose()? {
        println!("Message: {:#?}", message);
    }

    Ok(())
}
