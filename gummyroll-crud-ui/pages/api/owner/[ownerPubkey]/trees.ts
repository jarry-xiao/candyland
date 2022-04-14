import type { NextApiRequest, NextApiResponse } from "next";
import getTreesForAuthority from "../../../../lib/loaders/getTreesForAuthority";

export default async function handler(
  req: NextApiRequest,
  res: NextApiResponse
) {
  res.status(200).json({
    data: await getTreesForAuthority(req.query["ownerPubkey"] as string),
    status: 200,
    success: true,
  });
}
