import { Controller } from "@hotwired/stimulus";

import type { HtmxAfterRequestEvent } from "../htmx-types";
import { jsUtils } from "../utils/js-utils";
import { defineTargets } from "../utils/stimulus-utils";

// eslint-disable-next-line @typescript-eslint/unbound-method -- This is fine.
const { targets, getTarget } = defineTargets({
  textarea: "textarea",
  submit: "button",
});

export class AddCommentController extends Controller<HTMLFormElement> {
  public static identifier = "add-comment";
  public static targets = targets;
  getTarget = getTarget;

  override connect(): void {
    const $form = this.element;
    const $textarea = this.getTarget("textarea");
    const $submit = this.getTarget("submit");

    this.updateButtonVisibility();

    $textarea.addEventListener("keydown", (evt) => {
      const visible = this.updateButtonVisibility();

      // Use Ctrl+Enter to submit the form.
      if (visible && evt.ctrlKey && evt.key == "Enter") {
        $submit.click();
      }
    });

    $form.addEventListener("htmx:afterRequest", (event_) => {
      const event = event_ as HtmxAfterRequestEvent;
      if (event.detail.successful) {
        this.resetTextarea();
      }
    });
  }

  private updateButtonVisibility(): boolean {
    const $textarea = this.getTarget("textarea");
    const $submit = this.getTarget("submit");

    const text = $textarea.value;
    if (text.trim().length > 0) {
      jsUtils.show($submit);
      return true;
    } else {
      jsUtils.hide($submit);
      return false;
    }
  }

  private resetTextarea(): void {
    const $form = this.element;
    const $textarea = this.getTarget("textarea");

    $form.reset();
    $textarea.blur();
    this.updateButtonVisibility();
  }
}
