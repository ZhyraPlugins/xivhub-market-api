use sqlx::PgPool;

use crate::entities::ItemInfo;

pub async fn fetch_item_info(id: i32, db: &PgPool) -> Result<ItemInfo, sqlx::Error> {
    sqlx::query_as!(ItemInfo, "SELECT * from item_info where item_id = $1", id)
        .fetch_one(db)
        .await
}
