#![forbid(unsafe_code)]
#![deny(warnings)]
#![deny(clippy::missing_const_for_fn)]
#![deny(clippy::nursery)]
#![deny(clippy::pedantic)]
#![allow(clippy::missing_errors_doc)]
#![allow(clippy::module_name_repetitions)]
#![allow(clippy::missing_panics_doc)]
#![allow(clippy::cognitive_complexity)]

use ironworks::{excel::Excel, ffxiv, sqpack::SqPack, Ironworks};
use ironworks_sheets::{for_type, sheet};
use tracing::info;

/*
    Stats as of 26/02/2023

    2023-02-26T13:47:48.027050Z  INFO stats: max_item_name_length = 88
    2023-02-26T13:47:48.027279Z  INFO stats: max_description_length = 763
    2023-02-26T13:47:48.027450Z  INFO stats: max_item_kind_name_length = 17
    2023-02-26T13:47:48.027585Z  INFO stats: max_search_category_name_length = 30
*/

// Tool analyze the game data, currently used to know string length bounds.
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

    let mut max_item_name_length = 0;
    let mut max_description_length = 0;
    let mut max_item_kind_name_length = 0;
    let mut max_search_category_name_length = 0;

    for id in 0.. {
        if let Ok(item) = items_sheet.row(id) {
            let name = item.name.to_string();
            if !item.is_untradable && !name.is_empty() {
                let search_category =
                    item_search_category_sheet.row(item.item_search_category.into())?;
                let ui_category = item_ui_category_sheet.row(item.item_ui_category.into())?;

                let search_name = search_category.name.to_string();

                if search_name.trim().is_empty() {
                    continue;
                }

                max_item_name_length = max_item_name_length.max(name.len());
                max_description_length =
                    max_description_length.max(item.description.to_string().len());
                max_item_kind_name_length = max_item_kind_name_length
                    .max(order_major_to_str(ui_category.order_major.into()).len());
                max_search_category_name_length =
                    max_search_category_name_length.max(search_category.name.to_string().len());
            }
        } else {
            break;
        }
    }

    info!("max_item_name_length = {max_item_name_length}");
    info!("max_description_length = {max_description_length}");
    info!("max_item_kind_name_length = {max_item_kind_name_length}");
    info!("max_search_category_name_length = {max_search_category_name_length}");

    Ok(())
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
