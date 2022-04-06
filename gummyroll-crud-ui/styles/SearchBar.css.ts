import { style } from "@vanilla-extract/css";

export const accountControls = style({
  display: "flex",
  gap: 4,
});

export const header = style({
  display: "flex",
  gap: 12,
  padding: 16,
});

export const inputRoot = style({
  height: "100%",
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
  fontSize: ".8rem",
  width: "100%",
});

export const inputHint = style({
  color: "darkgray",
  fontSize: "0.8rem",
  paddingLeft: 12,
  margin: "0 0 0 16px",
});

export const searchForm = style({ flexGrow: 1 });
