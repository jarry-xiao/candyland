import React from "react";
import * as styles from "../styles/Button.css";

export default React.forwardRef(function Button(
  props: React.ComponentProps<"button">,
  ref: React.ForwardedRef<HTMLButtonElement>
) {
  return <button {...props} className={styles.button} ref={ref} />;
});
