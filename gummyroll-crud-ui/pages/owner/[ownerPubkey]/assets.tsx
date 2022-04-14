import { ImageList, ImageListItem } from "@mui/material";
import type { NextPage, NextPageContext } from "next";
import Link from "next/link";
import { useRouter } from "next/router";
import OwnerAsset from "../../../components/OwnerAsset";
import getAssetsForOwner from "../../../lib/loaders/getAssetsForOwner";
import Button from "../../../components/Button";
import { useWallet } from "@solana/wallet-adapter-react";
import useSWRImmutable from "swr/immutable";
import { unstable_serialize } from "swr";
import { AssetPayload } from "../../../lib/loaders/AssetTypes";

const OwnerassetsList: NextPage = () => {
  const router = useRouter();
  const { publicKey } = useWallet();
  const ownerPubkey = router.query.ownerPubkey;
  const { data: assets } = useSWRImmutable<
    Awaited<ReturnType<typeof getAssetsForOwner>>
  >(["owner", ownerPubkey, "assets"]);
  if (!assets || assets.length === 0) {
    return <h1>No assets</h1>;
  }
  return (
    <div style={{margin: '20px'}}>
      <h1>{ownerPubkey}&apos;s assets</h1>
      <ImageList cols={4} gap={16}>
        {assets.map((asset) => (
          <ImageListItem key={`${asset.treeAccount}:${asset.index}`}>
            <OwnerAsset {...asset} />
          </ImageListItem>
        ))}
      </ImageList>
    </div>
  );
};

export async function getInitialProps({ query }: NextPageContext) {
  const ownerPubkey = query.ownerPubkey as NonNullable<
    typeof query.ownerPubkey
  >[number];
  const assets = await getAssetsForOwner(ownerPubkey);
  if (!assets) {
    return { notFound: true };
  }
  const serverData = {
    [unstable_serialize(["owner", ownerPubkey, "assets"])]:
      assets as AssetPayload[],
  };
  return {
    props: {
      serverData,
    },
  };
}

export default OwnerassetsList;
