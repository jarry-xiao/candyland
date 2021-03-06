import React from "react";
import HashImage from "./HashImage";
import { CircularProgress } from "@mui/material";

type Props = Readonly<{
  data: string;
  treeAccount: string;
}>;

export default function AssetImage({ data, treeAccount }: Props) {
  const key = `${data}:${treeAccount}`;
  return (
    <React.Suspense fallback={<CircularProgress size="1.5rem" />}>
      <HashImage data={key} title={data}/>
    </React.Suspense>
  );
}
