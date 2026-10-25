import daisyuiPlugin from "daisyui";

const darkMode = "night";

/** @type {import('daisyui').Config} */
const daisyui = {
  // [daisyui-themes]
  themes: [darkMode, "nord"],
};

/** @type {import('tailwindcss').Config} */
export default {
  darkMode: ["selector", `[data-theme="${darkMode}"]`],
  // Includes the TypeScript sources for classes added at runtime, e.g. the
  // gallery slides.
  content: ["templates/**/*.html", "src/**/*.rs", "assets/js/**/*.ts"],
  theme: { extend: {} },
  plugins: [daisyuiPlugin],
  daisyui,
};
