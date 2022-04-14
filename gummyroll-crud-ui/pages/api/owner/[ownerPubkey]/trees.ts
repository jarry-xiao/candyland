import type { NextApiRequest, NextApiResponse } from "next";
import getTreesForAuthority from "../../../../lib/loaders/getTreesForAuthority";
import getClient from "../../../../lib/db/getClient";

export default async function handler(
  req: NextApiRequest,
  res: NextApiResponse
) {
  const client = await getClient();
  const results = await client?.query("SELECT * from cl_items;");
  res.status(200).json({
    data: await getTreesForAuthority(req.query["ownerPubkey"] as string),
    status: 200,
    success: true,
  });
}
