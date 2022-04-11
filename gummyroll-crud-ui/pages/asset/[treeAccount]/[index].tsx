import { useRouter } from "next/router";
import { NextPage, NextPageContext } from "next/types";
import useSWRImmutable from "swr/immutable";
import { unstable_serialize } from "swr";
import AssetImage from "../../../components/AssetImage";
import getAsset from "../../../lib/loaders/getAsset";
import { AssetPayload } from "../../../lib/loaders/AssetTypes";
import BufferData from "../../../components/BufferData";
import * as styles from "../../../styles/AssetDetails.css";

const AssetDetail: NextPage = () => {
  const router = useRouter();
  const index = router.query.index as NonNullable<
    typeof router.query.treeAccount
  >[number];
  const treeAccount = router.query.treeAccount as NonNullable<
    typeof router.query.treeAccount
  >[number];
  const { data } = useSWRImmutable<Awaited<ReturnType<typeof getAsset>>>([
    "asset",
    treeAccount,
    index,
  ]);
  if (!data) {
    return null;
  }
  const { data: assetData, owner } = data!;
  return (
    <>
      <h1>
        Asset {treeAccount}/{index} belonging to {owner}
      </h1>
      <div className={styles.imageContainer}>
        <div className={styles.imageWrapper}>
          <AssetImage data={assetData} treeAccount={treeAccount} />
        </div>
      </div>
      <p>Data</p>
      <BufferData buffer={Buffer.from(assetData)} />
    </>
  );
};

export async function getInitialProps({ query }: NextPageContext) {
  const index = query.index as NonNullable<typeof query.index>[number];
  const treeAccount = query.treeAccount as NonNullable<
    typeof query.treeAccount
  >[number];
  const asset = await getAsset(treeAccount, parseInt(index, 10));
  if (!asset) {
    return { notFound: true };
  }
  const serverData = {
    [unstable_serialize(["asset", treeAccount, index])]: asset as AssetPayload,
  };
  return {
    props: { serverData },
  };
}

export default AssetDetail;
