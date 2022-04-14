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