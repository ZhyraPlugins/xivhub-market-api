-- Add migration script here


create index if not exists idx_purchase_purchase_time on purchase( date(timezone('UTC', purchase_time)) );
create index if not exists idx_upload_upload_time on upload( date(timezone('UTC', upload_time)) );
