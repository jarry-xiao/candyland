import { style } from "@vanilla-extract/css";

export const button = style({
  appearance: "none",
  backgroundColor: "darkgray",
  borderColor: "lightgray",
  borderRadius: 8,
  borderStyle: "solid",
  borderWidth: 1,
  boxSizing: "border-box",
  color: "white",
  fontWeight: "bold",
  height: 44,
  minHeight: 44,
  padding: "0 12px",
  textTransform: "uppercase",
  ":hover": {
    backgroundColor: "gray",
  },
  ":active": {
    backgroundColor: "black",
  },
  ":disabled": {
    backgroundColor: "lightgray",
    cursor: "not-allowed",
  },
});
