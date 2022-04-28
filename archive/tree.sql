with tree as (
    select
        R.level,
        R.tree,
        R.node_idx,
        max(R.seq) as seq
    from
        cl_items as R
    group by
        R.level,
        R.tree,
        R.node_idx
),
     merkle as (
         select
             T.level,
             T.tree,
             T.node_idx,
             R.hash,
             R.seq
         from
             tree as T
                 inner join cl_items R on R.node_idx = T.node_idx
                 and R.seq = T.seq
     )

    select distinct on (node_idx) * from cl_items order by node_idx, seq, level desc ;
-- select *
-- from merkle


with node as (select level, node_idx from cl_items where node_idx = 16385 order by seq desc limit 1)
select distinct on (c.node_idx) * from cl_items as c, node as n where tree = decode('6128F38A464BD1B1D60BD116CAE0E6A13A9913E0DA6F1FBA78EBCE4D85F999E5', 'hex') AND c.level > n.level order by c.node_idx, c.seq, c.level desc



