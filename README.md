# market.xivhub.org api

This api is used to upload and list market data from FFXIV.

You can use the dalamud plugin to automatically upload data: https://github.com/ZhyraPlugins/MarketUploader


Current api:

```
POST /upload
# Upload listings

POST /history
# Upload purchases

GET /item/:id
# Get item listings

GET /item/:id/purchases
# Get item purchases

Query:
page - Starting from 0, max entries per page: 250

```
