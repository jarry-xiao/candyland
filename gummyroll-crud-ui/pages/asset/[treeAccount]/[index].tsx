import { useRouter } from "next/router";
import { NextPage, NextPageContext } from "next/types";
import useSWRImmutable from "swr/immutable";
import { unstable_serialize, useSWRConfig } from "swr";
import AssetImage from "../../../components/AssetImage";
import getAsset from "../../../lib/loaders/getAsset";
import BufferData from "../../../components/BufferData";
import * as styles from "../../../styles/AssetDetails.css";
import Button from "../../../components/Button";
import { useCallback, useState } from "react";
import removeAsset from "../../../lib/mutations/removeAsset";
import { useAnchorWallet } from "@solana/wallet-adapter-react";
import * as anchor from "@project-serum/anchor";
import { AssetPayload } from "../../../lib/loaders/AssetTypes";
import transferAsset from "../../../lib/mutations/transferAsset";

const AssetDetail: NextPage = () => {
  const router = useRouter();
  const anchorWallet = useAnchorWallet();
  const index = router.query.index as NonNullable<
    typeof router.query.treeAccount
  >[number];
  const treeAccount = router.query.treeAccount as NonNullable<
    typeof router.query.treeAccount
  >[number];
  const { mutate } = useSWRConfig();
  const { data } = useSWRImmutable<Awaited<ReturnType<typeof getAsset>>>([
    "asset",
    treeAccount,
    index,
  ]);
  const [isUndergoingMutation, setIsUndergoingMutation] = useState(false);
  const handleDestroyClick = useCallback(async () => {
    if (!anchorWallet || !data) {
      return;
    }
    setIsUndergoingMutation(true);
    try {
      const { treeAccount, treeAdmin } = data;
      const nodeIndex = parseInt(index, 10);
      const leafIndex = nodeIndex - (1 << Math.floor(Math.log2(nodeIndex)));
      await removeAsset(
        new anchor.web3.PublicKey(data.treeAccount),
        new anchor.web3.PublicKey(data.treeAdmin),
        nodeIndex,
        leafIndex, 
      );
      await Promise.all([
        mutate<AssetPayload>(
          unstable_serialize(["asset", treeAccount, index]),
          undefined
        ),
        mutate<AssetPayload[]>(
          unstable_serialize(["owner", data.owner, "assets"]),
          (currentAssets) =>
            currentAssets?.filter(
              (asset) =>
                asset.index !== parseInt(index, 10) &&
                asset.treeAccount !== treeAccount
            )
        ),
      ]);
      router.replace({
        pathname: "/owner/[ownerPubkey]/assets",
        query: {
          ownerPubkey: anchorWallet.publicKey.toBase58(),
        },
      });
    } finally {
      setIsUndergoingMutation(false);
    }
  }, [anchorWallet, data, index, mutate, router]);
  const handleTransferClick = useCallback(async () => {
    if (!anchorWallet || !data) {
      return;
    }
    setIsUndergoingMutation(true);
    try {
      const { treeAccount, treeAdmin, data: assetData } = data;
      const newOwnerBase58EncodedPubkey = window.prompt(
        "Please enter the base58-encoded public key of new owner:"
      );
      const newOwner = new anchor.web3.PublicKey(newOwnerBase58EncodedPubkey!);
      const owner = new anchor.web3.PublicKey(data.owner);
      const nodeIndex = parseInt(index, 10);
      const leafIndex = nodeIndex - (1 << Math.floor(Math.log2(nodeIndex)));
      console.log(nodeIndex);
      console.log(leafIndex);
      await transferAsset(
        new anchor.web3.PublicKey(data.treeAccount),
        new anchor.web3.PublicKey(data.treeAdmin),
        Buffer.from(assetData, "utf-8"),
        nodeIndex,
        leafIndex,
        owner,
        newOwner
      );
      await Promise.all([
        mutate<AssetPayload>(
          unstable_serialize(["asset", treeAccount, index]),
          undefined
        ),
        mutate<AssetPayload[]>(
          unstable_serialize(["owner", data.owner, "assets"]),
          (currentAssets) =>
            currentAssets?.filter(
              (asset) =>
                asset.index !== parseInt(index, 10) &&
                asset.treeAccount !== treeAccount
            )
        ),
      ]);
      router.replace({
        pathname: "/owner/[ownerPubkey]/assets",
        query: {
          ownerPubkey: anchorWallet.publicKey.toBase58(),
        },
      });
    } finally {
      setIsUndergoingMutation(false);
    }
  }, [anchorWallet, data, index, mutate, router]);
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
      {anchorWallet ? (
        <>
          <Button
            disabled={isUndergoingMutation}
            variant="primary"
            onClick={handleTransferClick}
          >
            Transfer
          </Button>
          <Button
            disabled={isUndergoingMutation}
            variant="danger"
            onClick={handleDestroyClick}
          >
            Destroy
          </Button>
        </>
      ) : null}
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
