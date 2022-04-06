import Link from "next/link";
import React from "react";
import ItemImage from "./ItemImage";

type Props = Readonly<{
  data: string;
  index: number;
  treeId: string;
}>;

export default function OwnerItem({ data, index, treeId }: Props) {
  return (
    <Link
      href={{ pathname: "/item/[treeId]/[index]", query: { index, treeId } }}
    >
      <a>
        <ItemImage data={data} treeId={treeId} />
      </a>
    </Link>
  );
}
