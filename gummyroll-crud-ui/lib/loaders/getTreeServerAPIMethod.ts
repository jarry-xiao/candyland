import getTreeServerAPIURL from "./getTreeServerAPIURL";
import TreeServerNotConfiguredError from "./TreeServerNotConfiguredError";

export default async function getTreeServerAPIMethod<TResponse>(path: string) {
  const treeServerURL = getTreeServerAPIURL();
  if (!treeServerURL) {
    throw new TreeServerNotConfiguredError();
  }
  const url = new URL(path, treeServerURL);
  const response = await fetch(url.toString());
  if (response.ok) {
    const json = (await response.json()) as { data: TResponse };
    return json.data;
  } else {
    throw new Error(response.statusText);
  }
}
