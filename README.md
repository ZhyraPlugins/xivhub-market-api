# market.xivhub.org api

This api is used to upload and list market data from FFXIV.

You can use the dalamud plugin to automatically upload data: https://github.com/ZhyraPlugins/MarketUploader


Current api:

```
POST /upload
# Upload listings

POST /history
# Upload purchases

GET /item
# Get list of available items

- Query
page - Starting from 0, entries per page: 1000

GET /item/:id
# Get item listings

GET /item/:id/purchases
# Get item purchases

- Query
page - Starting from 0, entries per page: 250

GET /item/:id/uploads
# Get item upload dates

GET /stats
# General stats

GET /last_uploads
# Last 250 uploads

```

## Setup

Needs the following env vars:

```
DATABASE_URL=postgres://user:pass@localhost/dbname
RUST_LOG=debug,sqlx=error,hyper=info
PORT=3000
XIVAPI_PRIVATE_KEY="xivapi.com key"
```
