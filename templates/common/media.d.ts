import { Fancybox } from "@fancyapps/ui";

declare global {
  interface Window {
    // TODO: Investigate why this is inferred as any :(
    Fancybox: typeof Fancybox;
  }
}

export {};
