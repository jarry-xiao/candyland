CREATE TABLE raw_txn
(
    signature varchar(64) PRIMARY KEY,
    slot      bigint not null,
    processed bool   not null
);

CREATE INDEX raw_slot on raw_txn (slot);

CREATE TABLE cl_items
(
    id       bigserial PRIMARY KEY,
    tree     BYTEA  NOT NULL,
    node_idx BIGINT NOT NULL,
    seq      BIGINT NOT NULL,
    level    BIGINT NOT NULL,
    hash     BYTEA  NOT NULL
);
-- Index All the things space is cheap
CREATE INDEX cl_items_tree_idx on cl_items (tree);
CREATE INDEX cl_items_hash_idx on cl_items (hash);
CREATE INDEX cl_items_level on cl_items (level);
CREATE INDEX cl_items_node_idx on cl_items (node_idx);
CREATE INDEX cl_items_uniq_operation_idx on cl_items (tree, level, seq);
CREATE UNIQUE INDEX cl_items__tree_node on cl_items (tree, node_idx);

-- START NFT METADATA
CREATE TYPE owner_type AS ENUM ('unknown', 'token', 'single');
CREATE TYPE royalty_target_type AS ENUM ('unknown', 'creators', 'fanout', 'single');
CREATE TYPE chain_mutability AS ENUM ('unknown', 'mutable', 'immutable');
CREATE TYPE mutability AS ENUM ('unknown', 'mutable', 'immutable');

create table asset_data
(
    id                    bigserial PRIMARY KEY,
    chain_data_mutability chain_mutability not null default 'mutable',
    schema_version        int              not null default 1,
    chain_data            jsonb            not null,
    metadata_url          varchar(200)     not null,
    metadata_mutability   mutability       not null default 'mutable',
    metadata              jsonb            not null
);

create table asset
(
    id                  bytea PRIMARY KEY,
    owner               bytea               not null,
    owner_type          owner_type          not null default 'single',
    -- delegation
    delegate            bytea,
    -- freeze
    frozen              bool                not null default false,
    -- supply
    supply              bigint              not null default 1,
    supply_mint         bytea,
    -- compression
    compressed          bool                not null default false,
    -- -- Can this asset be compressed
    compressible        bool                not null default false,
    tree_id             bytea,
    leaf                bytea,
    nonce               bigint              not null,
    -- royalty
    royalty_target_type royalty_target_type not null default 'creators',
    royalty_target      bytea,
    royalty_amount      int                 not null default 0,
    -- data
    chain_data_id       bigint references asset_data (id),
    -- visibility
    created_at          timestamp with time zone default (now() at time zone 'utc'),
    burnt_at            timestamp with time zone
);

create index asset_tree on asset (tree_id);
create index asset_leaf on asset (leaf);
create index asset_tree_leaf on asset (tree_id, leaf);
create index asset_revision on asset (tree_id, leaf, nonce);
create index asset_owner on asset (owner);
create index asset_delegate on asset (delegate);

-- grouping
create table asset_grouping
(
    id          bigserial PRIMARY KEY,
    asset_id    bytea references asset (id) not null,
    group_key   text                        not null,
    group_value text                        not null
);
-- Limit indexable grouping keys, meaning only create on specific keys, but index the ones we allow
create index asset_grouping_key on asset_grouping (group_key, group_value);
create index asset_grouping_value on asset_grouping (group_key, asset_id);

-- authority
create table asset_authority
(
    id        bigserial PRIMARY KEY,
    asset_id  bytea references asset (id) not null,
    scopes    text[],
    authority bytea                       not null
);
create index asset_authority_idx on asset_authority (asset_id, authority);

-- creators
create table asset_creators
(
    id       bigserial PRIMARY KEY,
    asset_id bytea references asset (id) not null,
    creator  bytea                       not null,
    share    int                         not null default 0,
    verified bool                        not null default false
);


create index asset_creator on asset_creators (asset_id, creator);
create index asset_verified_creator on asset_creators (asset_id, verified);
