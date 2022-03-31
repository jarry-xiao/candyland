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
                          inner join M on I.seq = M.mseq and I.tree = M.tree
                          where (1::integer << (21 - I.level::integer)) + (M.leaf_idx::integer >> I.level::integer) in (2103869, 1051935, 525966, 262982, 131490, 65744, 32873, 16437, 8219, 4108, 2055, 1026, 512, 257, 129, 65, 33, 17, 9, 5, 3)),
     tree as (select R.level, R.tree, R.node_idx, max(R.seq) as seq
              from rawTree as R
              group by R.level, R.tree, R.node_idx),
     merkle as (select T.level, T.tree, T.node_idx, R.hash, R.seq
                from tree as T
                         inner join rawTree R on R.node_idx = T.node_idx and R.seq = T.seq)
select *
from merkle



