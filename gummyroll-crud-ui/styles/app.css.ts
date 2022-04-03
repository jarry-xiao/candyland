import { globalStyle, style } from "@vanilla-extract/css";

globalStyle("html,body", {
  margin: 0,
  padding: 0,
});

export const shell = style({
  bottom: 0,
  left: 0,
  position: "absolute",
  right: 0,
  top: 0,
  display: "flex",
  flexDirection: "column",
});
