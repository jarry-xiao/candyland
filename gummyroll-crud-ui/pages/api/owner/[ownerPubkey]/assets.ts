import type { NextApiRequest, NextApiResponse } from "next";
import getAssetsForOwner from "../../../../lib/loaders/getAssetsForOwner";

export default async function handler(
  req: NextApiRequest,
  res: NextApiResponse
) {
  res.status(200).json({
    data: await getAssetsForOwner(req.query["ownerPubkey"] as string),
    status: 200,
    success: true,
  });
}
