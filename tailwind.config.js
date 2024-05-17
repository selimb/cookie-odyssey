/** @type {import('daisyui').Config} */
const daisyui = {
  // [daisyui-themes]
  themes: ["night", "pastel"],
};

/** @type {import('tailwindcss').Config} */
export default {
  content: ["templates/**/*.html", "src/**/*.rs"],
  theme: {
    extend: {},
  },
  plugins: [require("daisyui")],
  daisyui,
};
