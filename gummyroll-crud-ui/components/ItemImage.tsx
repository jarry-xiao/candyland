import React from "react";
import HashImage from "./HashImage";
import { CircularProgress } from "@mui/material";

type Props = Readonly<{
  data: string;
  treeId: string;
}>;

export default function ItemImage({ data, treeId }: Props) {
  const key = `${data}:${treeId}`;
  return (
    <React.Suspense fallback={<CircularProgress size="1.5rem" />}>
      <HashImage data={key} />
    </React.Suspense>
  );
}
