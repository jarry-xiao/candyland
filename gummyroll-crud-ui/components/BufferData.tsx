import * as styles from "../styles/BufferData.css";

type Props = Readonly<{
  buffer: Buffer;
}>;

export default function BufferData({ buffer }: Props) {
  return (
    <pre className={styles.root}>
      {buffer.toString("hex").match(/\d{2}/g)?.join("  ")}
    </pre>
  );
}
