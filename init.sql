CREATE TABLE cl_items
(
    id       serial PRIMARY KEY,
    tree     BYTEA  NOT NULL,
    node_idx BIGINT NOT NULL,
    seq      BIGINT NOT NULL,
    level    BIGINT NOT NULL,
    hash     BYTEA  NOT NULL
);
-- Index All the things space is cheap
CREATE INDEX cl_items_tree_idx ON cl_items (tree);
CREATE INDEX cl_items_hash_idx ON cl_items (hash);
CREATE INDEX cl_items_level ON cl_items (level);
CREATE INDEX cl_items_node_idx ON cl_items (node_idx);
CREATE INDEX cl_items_uniq_operation_idx ON cl_items (tree, level, seq);
CREATE INDEX cl_items__tree_node ON cl_items (tree, node_idx);

-- START CRUD
create table app_specific
(
    leaf    bytea not null,
    msg     text PRIMARY KEY,
    tree_id bytea not null,
    owner   bytea not null,
    revision bigint not null
);

CREATE INDEX app_specific_idx_owner ON app_specific (owner);
CREATE INDEX app_specific_idx_tree_id ON app_specific (tree_id);

create table app_specific_ownership
(
    authority bytea not null,
    tree_id bytea not null primary key
);

create index app_specific_shizzle_mynizzle On app_specific_ownership (authority);
-- END CRUD

-- START NFT METADATA
create table nft_uuid
(
    nonce uuid PRIMARY KEY,
    tree_id bytea not null,
    leaf bytea not null,
    revision bigint not null
);

create table nft_metadata
(
    owner      bytea not null,
    delegate   bytea,
    nonce      uuid PRIMARY KEY,
    revision   bigint not null,

    name       text not null,
    symbol     text not null,
    uri        text not null,
    sellerFeeBasisPoints int,
    primarySaleHappened boolean,
    isMutable boolean
);

create table nft_creators (
    nonce uuid PRIMARY KEY,
    creator bytea not null,
    revision bigint not null
);

create index nft_uuid_uuid On nft_uuid (tree_id);
create index nft_uuid_leaf On nft_uuid (leaf);
create index nft_uuid_nonce On nft_uuid (nonce);
create index nft_uuid_revision On nft_uuid (revision);

create index nft_metadata_owner On nft_metadata (owner);
create index nft_metadata_delegate On nft_metadata (delegate);

create index nft_creators_nonce On nft_creators (nonce);
create index nft_creators_creator On nft_creators (creator);
create index nft_creators_revision On nft_creators (revision);

-- To Be Added:
--    editionNonce bigint not null,
--    tokenStandard: null,
--     tokenProgramVersion: {
--     original: {},
--     },
--     collections: null,
--     uses: null,
--     creators: [],
