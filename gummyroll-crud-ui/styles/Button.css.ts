import { style, styleVariants } from "@vanilla-extract/css";

const base = style({
  appearance: "none",
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
  ":active": {
    backgroundColor: "black",
  },
  ":disabled": {
    backgroundColor: "lightgray",
    borderColor: "lightgray",
    cursor: "not-allowed",
  },
});

export const variant = styleVariants({
  default: [
    base,
    {
      backgroundColor: "darkgray",
      borderColor: "lightgray",
      selectors: {
        "&:hover:not(:active, :disabled)": {
          backgroundColor: "gray",
        },
      },
    },
  ],
  primary: [
    base,
    {
      backgroundColor: "green",
      borderColor: "lightgreen",
      selectors: {
        "&:hover:not(:active, :disabled)": {
          backgroundColor: "darkgreen",
        },
      },
    },
  ],
});
