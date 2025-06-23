declare global {
  export type Toast = {
    message: string;
    detail?: string;
    error?: unknown;
    variant: "success" | "error";
    auto_close?: boolean;
  };

  interface Window {
    toast(data: Toast): void;
  }
}

export {};
