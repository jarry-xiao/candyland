export type AssetPayload = Readonly<{
  data: string;
  index: number;
  owner: string;
  treeAccount: string;
  treeAdmin: string;
}>;

export type AssetProof = {
  hash: string;
  proof: string[];
  root: string;
};
