import { Client } from "ts-postgres";

let client: Client | null;
export default async function getClient() {
  if (!client) {
    client = new Client({
      database: process.env.PGSQL_DATABASE!,
      host: process.env.PGSQL_HOST!,
      keepAlive: true,
      password: process.env.PGSQL_PASSWORD!,
      user: process.env.PGSQL_USER!,
    });
    client.on("end", () => {
      client = null;
    });
    await client.connect();
  }
  return client;
}
