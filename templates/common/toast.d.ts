declare global {
  export type Toast = {
    message: string;
    detail?: string;
    variant: "success" | "error";
    auto_close?: boolean;
  };

  interface Window {
    toast(data: Toast): void;
  }
}

export {};
