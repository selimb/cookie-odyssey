declare global {
  type Theme = "light" | "dark";
  interface Window {
    theme: {
      value: Theme;
      onChange: (fn: (value: Theme) => void) => void;
    };
  }
}

export {};
