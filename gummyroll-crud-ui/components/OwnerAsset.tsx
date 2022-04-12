import Link from "next/link";
import React from "react";
import AssetImage from "./AssetImage";

type Props = Readonly<{
  data: string;
  index: number;
  treeAccount: string;
}>;

export default function OwnerAsset({ data, index, treeAccount }: Props) {
  return (
    <Link
      href={{
        pathname: "/asset/[treeAccount]/[index]",
        query: { index, treeAccount },
      }}
    >
      <a>
        <AssetImage data={data} treeAccount={treeAccount} />
      </a>
    </Link>
  );
}
