import Link from "next/link";
import React from "react";
import ItemImage from "./ItemImage";

type Props = Readonly<{
  data: string;
  index: number;
  treeAccount: string;
}>;

export default function OwnerItem({ data, index, treeAccount }: Props) {
  return (
    <Link
      href={{
        pathname: "/item/[treeAccount]/[index]",
        query: { index, treeAccount },
      }}
    >
      <a>
        <ItemImage data={data} treeAccount={treeAccount} />
      </a>
    </Link>
  );
}
