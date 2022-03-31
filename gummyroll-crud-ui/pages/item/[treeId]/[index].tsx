import InferNextPropsType from "infer-next-props-type";
import { NextPage, NextPageContext } from "next/types";
import ItemImage from "../../../components/ItemImage";
import getItem from "../../../lib/loaders/getItem";

const ItemDetail: NextPage<InferNextPropsType<typeof getServerSideProps>> = ({
  item: { data, index, treeId },
}) => {
  return (
    <>
      <h1>
        Item {treeId}/{index}
      </h1>
      <ItemImage data={data} treeId={treeId} />
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
  return {
    props: { item },
  };
}

export default ItemDetail;
