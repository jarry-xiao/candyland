import React from "react";
import ItemImage from "./ItemImage";

type Props = Readonly<{
  data: string;
  treeId: string;
}>;

export default function OwnerItem({ data, treeId }: Props) {
  return <ItemImage data={data} treeId={treeId} />;
}
