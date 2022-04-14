let url: URL | null | undefined;
export default function getTreeServerAPIURL() {
  if (url === undefined) {
    const config = process.env.NEXT_PUBLIC_TREE_SERVER_API_ENDPOINT;
    if (!config) {
      url = null;
    } else {
      url = new URL(config);
    }
  }
  return url;
}
