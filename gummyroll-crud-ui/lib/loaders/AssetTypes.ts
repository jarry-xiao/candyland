export type AssetPayload = Readonly<{
  data: string;
  index: number;
  owner: string;
  treeAccount: string;
}>;

export type AssetProof = Readonly<{
  hash: number[];
  proof: number[][];
  root: number[];
}>;
