with M as (
    SELECT max(seq) as mseq, leaf_idx, tree
    From cl_meta
    GROUP BY tree, leaf_idx
),
     rawTree as (SELECT I.level,
                        I.tree,
                        I.seq,
                        M.leaf_idx,
                        (1::integer << (21 - I.level::integer)) + (M.leaf_idx::integer >> I.level::integer) as node_idx,
                        I.hash
                 from cl_items as I
                          inner join M on I.seq = M.mseq and I.tree = M.tree),
     tree as (select R.level, R.tree, R.node_idx, max(R.seq) as seq
              from rawTree as R
              group by R.level, R.tree, R.node_idx),
     merkle as (select T.level, T.tree, T.node_idx, R.hash, R.seq
                from tree as T
                         inner join rawTree R on R.node_idx = T.node_idx and R.seq = T.seq)
select *
from merkle



