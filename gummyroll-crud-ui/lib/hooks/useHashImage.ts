import { useEffect, useState } from "react";

const hashprint = require("hashprintjs");

type DataURL = string;

const cache: Record<
  string,
  | void
  | { __type: "promise"; promise: Promise<void> }
  | { __type: "result"; url: DataURL }
> = {};

export default function useHashImage(data: string): DataURL {
  const cacheEntry = cache[data];
  if (cacheEntry === undefined) {
    const promise = new Promise<void>(async (resolve) => {
      console.log(data);
      const url = (await hashprint({ data })) as DataURL;
      cache[data] = { __type: "result", url };
      resolve();
    });
    cache[data] = { __type: "promise", promise };
    throw promise;
  } else if (cacheEntry.__type === "promise") {
    throw cacheEntry.promise;
  }
  return cacheEntry.url;
}
