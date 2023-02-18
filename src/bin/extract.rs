#![forbid(unsafe_code)]
#![deny(warnings)]
#![deny(clippy::missing_const_for_fn)]
#![deny(clippy::nursery)]
#![deny(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_panics_doc)]

use ironworks::{excel::Excel, ffxiv, sqpack::SqPack, Ironworks};
use ironworks_sheets::{for_type, sheet};
use std::{io::Write, time::Instant};
use tracing::info;
use xivhub_market::entities::ItemInfo;

// Tool to import game data and store it in a better format. Requires the game to be installed.
fn main() -> color_eyre::Result<()> {
    dotenvy::dotenv().ok();
    std::env::set_var(
        "RUST_LOG",
        std::env::var("RUST_LOG").unwrap_or_else(|_| String::from("info")),
    );

    // initialize tracing
    tracing_subscriber::fmt::init();

    let ironworks =
        Ironworks::new().with_resource(SqPack::new(ffxiv::FsResource::search().unwrap()));

    // Read fields out of excel.
    let excel = Excel::with()
        .language(ffxiv::Language::English)
        .build(&ironworks, ffxiv::Mapper::new());

    let items_sheet = excel.sheet(for_type::<sheet::Item>())?;
    let item_search_category_sheet = excel.sheet(for_type::<sheet::ItemSearchCategory>())?;
    let item_ui_category_sheet = excel.sheet(for_type::<sheet::ItemUICategory>())?;

    let mut id = 0;
    // I expect there to be about 15k items (after filtering)
    let mut items: Vec<ItemInfo> = Vec::with_capacity(15000);

    let now = Instant::now();

    while let Ok(item) = items_sheet.row(id) {
        let name = item.name.to_string();
        if !item.is_untradable && !name.is_empty() {
            //info!(id, name, "parsing");
            let search_category =
                item_search_category_sheet.row(item.item_search_category.into())?;
            let ui_category = item_ui_category_sheet.row(item.item_ui_category.into())?;

            items.push(ItemInfo {
                item_id: id.try_into().unwrap(),
                name,
                icon: icon_id_to_url(item.icon, false),
                icon_hd: icon_id_to_url(item.icon, true),
                description: item.description.to_string(),
                item_kind_name: order_major_to_str(ui_category.order_major.into()).to_string(),
                item_kind_id: item.item_ui_category.into(),
                item_search_category: item.item_search_category.into(),
                item_search_category_iconhd: icon_id_to_url(
                    search_category.icon.try_into().unwrap(),
                    true,
                ),
                item_search_category_name: search_category.name.to_string(),
                stack_size: item.stack_size.try_into().unwrap(),
                level_item: item.level_item.try_into().unwrap(),
                level_equip: item.level_equip.into(),
                materia_slot_count: item.materia_slot_count.into(),
                rarity: item.rarity.into(),
                can_be_hq: item.can_be_hq,
            });
        }

        id += 1;
    }

    let elapsed = now.elapsed();
    info!("Got {} items in {elapsed:?}", items.len());

    let output = std::path::Path::new("items.bin.zstd");
    let file = std::fs::File::create(output)?;
    let mut enc = zstd::stream::Encoder::new(file, 10)?;

    bincode::serialize_into(&mut enc, &items)?;
    enc.flush()?;
    enc.finish()?;

    info!("Saved to items.bin.zstd");

    Ok(())
}

fn icon_id_to_url(id: u16, hd: bool) -> String {
    // xxyyy -> /i/0xx000/0xxyyy.png
    let x: String = id.to_string().chars().take(2).collect();
    let hd = if hd { "_hr1" } else { "" };
    format!("/i/0{x}000/0{id}{hd}.png")
}

const fn order_major_to_str(major: u32) -> &'static str {
    match major {
        1 => "Arms",
        2 => "Tools",
        3 => "Armor",
        4 => "Accesories",
        5 => "Medicines & Meals",
        6 => "Materials",
        7 => "Other",
        _ => "Unknown",
    }
}

#[cfg(test)]
mod tests {
    use crate::icon_id_to_url;

    #[test]
    fn icon_to_url() {
        assert_eq!(icon_id_to_url(29221, false), "/i/029000/029221.png");
    }
}
