/**
 * This plugin implementation for PostgreSQL requires the following tables
 */
-- The table storing accounts


CREATE TABLE cl_meta (
                         id serial PRIMARY KEY,
                         tree BYTEA NOT NULL,
                         leaf_idx BIGINT NOT NULL,
                         seq BIGINT NOT NULL UNIQUE
);

CREATE INDEX cl_meta_tree_idx ON cl_meta (tree);
CREATE INDEX cl_meta_leaf_index_idx ON cl_meta (leaf_idx);
CREATE INDEX cl_meta_uniq_operation_idx ON cl_meta (tree, leaf_idx, seq);
CREATE INDEX cl_meta_query_leaf_idx ON cl_meta (tree, leaf_idx);

CREATE TABLE cl_items (
     id serial PRIMARY KEY,
     tree BYTEA NOT NULL,
     seq BIGINT NOT NULL,
     level BIGINT NOT NULL,
     hash BYTEA NOT NULL
);

CREATE INDEX cl_items_tree_idx ON cl_items (tree);
CREATE INDEX cl_items_hash_idx ON cl_items (hash);
CREATE INDEX cl_items_uniq_operation_idx ON cl_items (tree, level, seq);

