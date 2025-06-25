import { Controller } from "@hotwired/stimulus";

import { defineTargets } from "./stimulus-utils";

export type ToastVariant = "error" | "success";

export type ToastData = {
  variant: ToastVariant;
  message: string;
  auto_close?: boolean;

  // Extra metadata. Only shown in console.
  detail?: string;
  error?: unknown;
};

// Keep in sync with [toast]
const TOAST_EVT = "app.toast" as const;

// eslint-disable-next-line @typescript-eslint/unbound-method -- This is fine.
const { targets, getTarget } = defineTargets({
  template: "template",
});

export class ToastController extends Controller {
  public static identifier = "toast";
  static targets = targets;
  getTarget = getTarget;

  override connect(): void {
    document.body.addEventListener(TOAST_EVT, this.onToastEvent);
  }

  override disconnect(): void {
    document.body.removeEventListener(TOAST_EVT, this.onToastEvent);
  }

  private onToastEvent: EventListener = (event: Event) => {
    const customEvent = event as CustomEvent<ToastData>;
    this.showToast(customEvent.detail);
  };

  private showToast(data: ToastData): void {
    const { $toast, $button } = this.makeToast(data);
    this.element.append($toast);

    const remove = (): void => {
      $toast.remove();
    };

    if (data.auto_close) {
      setTimeout(remove, 5000);
    }

    $button.addEventListener("click", remove);
  }

  private makeToast(data: ToastData): {
    $toast: HTMLDivElement;
    $button: HTMLButtonElement;
  } {
    const clone = this.getTarget("template").content.cloneNode(
      true,
    ) as DocumentFragment;

    const $toast = clone.querySelector("div");
    if (!$toast) throw new Error("Could not find toast div in template");

    const variant = data.variant;

    {
      const classNames =
        $toast.getAttribute(`data-${variant}-class`)?.split(" ") ?? [];
      $toast.classList.add(...classNames);
    }

    const $message = $toast.querySelector("#message");
    if (!$message) throw new Error("Could not find #message element in toast");
    $message.textContent = data.message;

    const $button = $toast.querySelector("button");
    if (!$button) throw new Error("Could not find close button in toast");

    {
      const classNames =
        $button.getAttribute(`data-${variant}-class`)?.split(" ") ?? [];
      $button.classList.add(...classNames);
    }

    return { $toast, $button };
  }
}

export function toast(data: ToastData): void {
  document.body.dispatchEvent(
    new CustomEvent<ToastData>(TOAST_EVT, { detail: data }),
  );
}
