/**
 * Inspired by https://web.dev/patterns/theming/theme-switch
 */

import { Controller } from "@hotwired/stimulus";
import { defineTargets } from "./stimulus-utils";

const STORAGE_KEY = "theme-preference";

type Theme = "light" | "dark";

// Matches [daisyui-themes]
const THEME_MAP = {
  light: "nord",
  dark: "night",
} satisfies Record<Theme, string>;

const THEME_TITLE_MAP = {
  light: "Toggle dark mode",
  dark: "Toggle light mode",
} satisfies Record<Theme, string>;

function getColorPreference(): Theme {
  const stored = localStorage.getItem(STORAGE_KEY);
  if (stored) return stored as Theme;
  else
    return window.matchMedia("(prefers-color-scheme: dark)").matches
      ? "dark"
      : "light";
}

const { targets, getTarget } = defineTargets({ input: "input" });

let THEME = {
  value: getColorPreference(),
};

export class ThemeToggleController extends Controller<HTMLLabelElement> {
  public static identifier = "theme-toggle";

  static targets = targets;
  getTarget = getTarget;

  connect() {
    const $themeInput = this.getTarget("input");

    this.reflectPreference(THEME.value);

    $themeInput.addEventListener("click", () => {
      // Flip current value.
      const currentTheme = THEME.value;
      const newTheme = currentTheme === "light" ? "dark" : "light";
      this.setPreference(newTheme);
    });

    window
      .matchMedia("(prefers-color-scheme: dark)")
      .addEventListener("change", ({ matches: isDark }) => {
        const newTheme = isDark ? "dark" : "light";
        this.setPreference(newTheme);
      });
  }

  private setPreference(theme: Theme) {
    THEME.value = theme;
    localStorage.setItem(STORAGE_KEY, theme);
    this.reflectPreference(theme);
  }

  private reflectPreference(theme: Theme) {
    document.documentElement.setAttribute("data-theme", THEME_MAP[theme]);
    this.element.title = THEME_TITLE_MAP[theme];
    this.getTarget("input").checked = theme === "dark";
  }
}
