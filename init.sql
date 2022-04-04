CREATE TABLE cl_meta
(
    id       serial PRIMARY KEY,
    tree     BYTEA  NOT NULL,
    leaf_idx BIGINT NOT NULL,
    seq      BIGINT NOT NULL UNIQUE
);
-- Index All the things space is cheap
CREATE INDEX cl_meta_tree_idx ON cl_meta (tree);
CREATE INDEX cl_meta_leaf_index_idx ON cl_meta (leaf_idx);
CREATE INDEX cl_meta_uniq_operation_idx ON cl_meta (tree, leaf_idx, seq);
CREATE INDEX cl_meta_query_leaf_idx ON cl_meta (tree, leaf_idx);

CREATE TABLE cl_items
(
    id       serial PRIMARY KEY,
    tree     BYTEA  NOT NULL,
    node_idx BIGINT,
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

create table app_specific
(
    leaf    varchar(32) not null,
    msg     text PRIMARY KEY,
    tree_id varchar(32) not null,
    owner   varchar(32) not null
);

CREATE INDEX app_specific_idx_owner ON app_specific (owner);
CREATE INDEX app_specific_idx_tree_id ON app_specific (tree_id);