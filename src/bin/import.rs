use std::time::Instant;

use sqlx::postgres::PgPoolOptions;
use tracing::info;
use xivhub_market::entities::ItemInfo;

#[tokio::main]
async fn main() -> color_eyre::Result<()> {
    run().await
}

async fn run() -> color_eyre::Result<()> {
    dotenvy::dotenv().ok();
    std::env::set_var(
        "RUST_LOG",
        std::env::var("RUST_LOG").unwrap_or_else(|_| String::from("info")),
    );

    // initialize tracing
    tracing_subscriber::fmt::init();

    std::fs::create_dir("assets").ok();
    let input = std::path::Path::new("assets/items.bin.zstd");
    let file = std::fs::File::open(input)?;
    let mut decoder = zstd::stream::Decoder::new(file)?;

    let items: Vec<ItemInfo> = bincode::deserialize_from(&mut decoder)?;

    info!("Loaded {} items from items.bin.zstd", items.len());

    let pool = PgPoolOptions::new()
        .max_connections(
            std::env::var("DATABASE_MAX_CONNECTIONS")
                .map(|x| x.parse().expect("valid number"))
                .unwrap_or(30),
        )
        .connect(&std::env::var("DATABASE_URL")?)
        .await?;

    let start = Instant::now();
    let tx = pool.begin().await?;

    sqlx::query!("DELETE FROM item_info").execute(&pool).await?;

    for item in items {
        sqlx::query!(
            "INSERT INTO item_info
            (item_id, name, icon, icon_hd, description, item_kind_name, item_kind_id, item_search_category,
            item_search_category_iconhd, item_search_category_name,
            stack_size, level_item, level_equip, materia_slot_count, rarity, can_be_hq)
            VALUES
            ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16)
            ",
            item.item_id,
            item.name,
            item.icon,
            item.icon_hd,
            item.description,
            item.item_kind_name,
            item.item_kind_id,
            item.item_search_category,
            item.item_search_category_iconhd,
            item.item_search_category_name,
            item.stack_size,
            item.level_item,
            item.level_equip,
            item.materia_slot_count,
            item.rarity,
            item.can_be_hq
        )
        .execute(&pool).await?;
    }

    tx.commit().await?;

    let elapsed = start.elapsed();
    info!("Done in {elapsed:?}");

    Ok(())
}
