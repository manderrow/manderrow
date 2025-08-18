use std::io::Write;

use anyhow::{Context as _, Result};

pub fn run(mut args: lexopt::Parser) -> Result<()> {
    let game = args.next()?.context("Missing required argument GAME")?;
    let game: String = match game {
        lexopt::Arg::Value(game) => game
            .into_string()
            .map_err(|_| anyhow::anyhow!("Invalid UTF-8"))?,
        _ => anyhow::bail!("{}", game.unexpected()),
    };

    let reqwest = crate::Reqwest(reqwest::Client::builder().build()?);

    let mod_index = tokio::runtime::Runtime::new()?.block_on(async {
        super::fetch_mod_index(None, &reqwest, &game, true, None).await?;

        super::read_mod_index(&game).await
    })?;

    let buf = super::query_mod_index_to_json(&mod_index, "", &[], 0, None)?;

    std::io::stdout().write_all(buf.as_bytes())?;

    Ok(())
}
