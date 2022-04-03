import { style } from "@vanilla-extract/css";

export const header = style({
  backgroundColor: "white",
  boxSizing: "border-box",
  left: 0,
  padding: 16,
  position: "fixed",
  right: 0,
  top: 0,
  zIndex: 1,
});

export const input = style({
  appearance: "none",
  borderWidth: 1,
  borderColor: "lightgray",
  borderStyle: "solid",
  borderRadius: 8,
  boxSizing: "border-box",
  height: "100%",
  padding: 12,
  width: "100%",
  fontSize: "1rem",
});

export const inputHint = style({
  color: "darkgray",
  fontSize: "0.8rem",
  paddingLeft: 12,
  margin: "6px 0 0 0",
});
