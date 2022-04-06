import { ImageList, ImageListItem } from "@mui/material";
import type { NextPage, NextPageContext } from "next";
import Link from "next/link";
import { useRouter } from "next/router";
import OwnerItem from "../../../components/OwnerItem";
import getItemsForOwner from "../../../lib/loaders/getItemsForOwner";
import Button from "../../../components/Button";
import { useWallet } from "@solana/wallet-adapter-react";
import useSWR, { unstable_serialize, useSWRConfig } from "swr";
import { ItemPayload } from "../../../lib/loaders/ItemTypes";

const OwnerItemsList: NextPage = () => {
  const router = useRouter();
  const { publicKey } = useWallet();
  const ownerPubkey = router.query.ownerPubkey;
  const { data: items } = useSWR<Awaited<ReturnType<typeof getItemsForOwner>>>([
    "owner",
    ownerPubkey,
    "items",
  ]);
  if (!items || items.length === 0) {
    return <h1>No items</h1>;
  }
  return (
    <>
      <h1>{ownerPubkey}&apos;s items</h1>
      <ImageList cols={4} gap={16}>
        {items.map((item) => (
          <ImageListItem key={`${item.treeAccount}:${item.index}`}>
            <OwnerItem {...item} />
          </ImageListItem>
        ))}
      </ImageList>
      {publicKey ? (
        <Link
          href={{
            pathname: "/owner/[ownerPubkey]/add",
            query: { ownerPubkey },
          }}
          passHref
        >
          <Button>Add</Button>
        </Link>
      ) : null}
    </>
  );
};

export async function getServerSideProps({ query }: NextPageContext) {
  const ownerPubkey = query.ownerPubkey as NonNullable<
    typeof query.ownerPubkey
  >[number];
  const items = await getItemsForOwner(ownerPubkey);
  if (!items) {
    return { notFound: true };
  }
  const serverData = {
    [unstable_serialize(["owner", ownerPubkey, "items"])]:
      items as ItemPayload[],
  };
  return {
    props: {
      serverData,
    },
  };
}

export default OwnerItemsList;
