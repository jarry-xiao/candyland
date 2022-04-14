export type AssetPayload = Readonly<{
  data: string;
  index: number;
  owner: string;
  treeAccount: string;
  treeAdmin: string;
}>;

export type AssetProof = Readonly<{
  hash: string;
  proof: string[];
  root: string;
}>;
