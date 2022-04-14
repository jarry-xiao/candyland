import type { NextApiRequest, NextApiResponse } from "next";
import getClient from "../../../../../lib/db/getClient";

export default async function handler(
  req: NextApiRequest,
  res: NextApiResponse
) {
  const client = await getClient();
  const results = await client!.query("SELECT * from cl_items;");
  res.status(200).json({ query: req.query });
}
