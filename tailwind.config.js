const darkMode = "night";

/** @type {import('daisyui').Config} */
const daisyui = {
  // [daisyui-themes]
  // FIXME narrow down
  themes: [darkMode, "nord"],
};

/** @type {import('tailwindcss').Config} */
export default {
  darkMode: ["selector", `[data-theme="${darkMode}"]`],
  content: ["templates/**/*.html", "src/**/*.rs"],
  theme: { extend: {} },
  plugins: [require("daisyui")],
  daisyui,
};
