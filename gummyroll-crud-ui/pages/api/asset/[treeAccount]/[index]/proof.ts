import type { NextApiRequest, NextApiResponse } from "next";

export default async function handler(
  req: NextApiRequest,
  res: NextApiResponse
) {
  res.status(200).json({
    data: {
      hash: "TODO",
      proof: ["TODO"],
      root: "TODO",
    },
    status: 200,
    success: true,
  });
}
