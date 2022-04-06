import { useRouter } from "next/router";
import { NextPage, NextPageContext } from "next/types";
import useSWR, { unstable_serialize } from "swr";
import ItemImage from "../../../components/ItemImage";
import getItem from "../../../lib/loaders/getItem";
import { ItemPayload } from "../../../lib/loaders/ItemTypes";

const ItemDetail: NextPage = () => {
  const router = useRouter();
  const index = router.query.index as NonNullable<
    typeof router.query.treeId
  >[number];
  const treeId = router.query.treeId as NonNullable<
    typeof router.query.treeId
  >[number];
  const { data } = useSWR<Awaited<ReturnType<typeof getItem>>>([
    "item",
    treeId,
    index,
  ]);
  if (!data) {
    return null;
  }
  const { data: itemData, owner } = data!;
  return (
    <>
      <h1>
        Item {treeId}/{index} belonging to {owner}
      </h1>
      <ItemImage data={itemData} treeId={treeId} />
    </>
  );
};

export async function getServerSideProps({ query }: NextPageContext) {
  const index = query.index as NonNullable<typeof query.index>[number];
  const treeId = query.treeId as NonNullable<typeof query.treeId>[number];
  const item = await getItem(treeId, parseInt(index, 10));
  if (!item) {
    return { notFound: true };
  }
  const serverData = {
    [unstable_serialize(["item", treeId, index])]: item as ItemPayload,
  };
  return {
    props: { serverData },
  };
}

export default ItemDetail;
