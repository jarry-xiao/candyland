import type { NextApiRequest, NextApiResponse } from "next";
import getAsset from "../../../../../lib/loaders/getAsset";

export default async function handler(
  req: NextApiRequest,
  res: NextApiResponse
) {
  res.status(200).json({
    data: await getAsset(
      req.query["treeAccount"] as string,
      parseInt(req.query["index"] as string, 10)
    ),
    status: 200,
    success: true,
  });
}
