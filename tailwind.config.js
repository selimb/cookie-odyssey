/** @type {import('daisyui').Config} */
const daisyui = {
  // [daisyui-themes]
  themes: ["sunset", "pastel"],
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
