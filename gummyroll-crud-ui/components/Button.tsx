import React from "react";
import * as styles from "../styles/Button.css";

type Props = Readonly<
  React.ComponentProps<"button"> & {
    variant?: "danger" | "primary";
  }
>;

export default React.forwardRef(function Button(
  props: Props,
  ref: React.ForwardedRef<HTMLButtonElement>
) {
  return (
    <button
      {...props}
      className={
        styles.variant[
          props.variant ?? (props.type === "submit" ? "primary" : "default")
        ]
      }
      ref={ref}
    />
  );
});
