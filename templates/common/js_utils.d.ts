declare global {
  interface Window {
    jsUtils: {
      once(key: string, fn: () => void): void;
      show($elem: Element): void;
      hide($elem: Element): void;
      observe(selector: string, fn: ($elem: Element) => void): void;
    };
  }
}

export {};
