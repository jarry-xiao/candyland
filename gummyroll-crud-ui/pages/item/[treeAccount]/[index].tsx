import { useRouter } from "next/router";
import { NextPage, NextPageContext } from "next/types";
import useSWRImmutable from "swr/immutable";
import { unstable_serialize } from "swr";
import ItemImage from "../../../components/ItemImage";
import getItem from "../../../lib/loaders/getItem";
import { ItemPayload } from "../../../lib/loaders/ItemTypes";
import BufferData from "../../../components/BufferData";

const ItemDetail: NextPage = () => {
  const router = useRouter();
  const index = router.query.index as NonNullable<
    typeof router.query.treeAccount
  >[number];
  const treeAccount = router.query.treeAccount as NonNullable<
    typeof router.query.treeAccount
  >[number];
  const { data } = useSWRImmutable<Awaited<ReturnType<typeof getItem>>>([
    "item",
    treeAccount,
    index,
  ]);
  if (!data) {
    return null;
  }
  const { data: itemData, owner } = data!;
  return (
    <>
      <h1>
        Item {treeAccount}/{index} belonging to {owner}
      </h1>
      <ItemImage data={itemData} treeAccount={treeAccount} />
      <p>Data</p>
      <BufferData buffer={Buffer.from(itemData)} />
    </>
  );
};

export async function getInitialProps({ query }: NextPageContext) {
  const index = query.index as NonNullable<typeof query.index>[number];
  const treeAccount = query.treeAccount as NonNullable<
    typeof query.treeAccount
  >[number];
  const item = await getItem(treeAccount, parseInt(index, 10));
  if (!item) {
    return { notFound: true };
  }
  const serverData = {
    [unstable_serialize(["item", treeAccount, index])]: item as ItemPayload,
  };
  return {
    props: { serverData },
  };
}

export default ItemDetail;
