const colors = require("tailwindcss/colors");

module.exports = {
  purge: [],
  darkMode: false, // or 'media' or 'class'
  theme: {
    // color palette: https://colors.muz.li/palette/361d32/543c52/f55951/edd2cb/f1e8e6
    colors: {
      transparent: "transparent",
      current: "currentColor",
      white: colors.coolGray[100],
      gray: colors.coolGray,
      black: colors.black,
      primary: {
        DEFAULT: "#21121f",
        light: "#4b3549",
      },
      accent: {
        DEFAULT: "#f4473e",
        light: "#f65d55",
      },
      light: {
        DEFAULT: "#edd2cb",
        light: "#f1e8e6",
      },
    },
    minHeight: {
      0: "0",
      "1/4": "25%",
      "1/2": "50%",
      "3/4": "75%",
      full: "100%",
    },
    extend: {
      maxHeight: {
        144: "36rem",
        192: "48rem",
      },
    },
  },
  variants: {
    extend: {},
  },
  plugins: [],
};
