import { Controller } from "@hotwired/stimulus";

import { jsUtils } from "../js-utils";
import { defineTargets } from "../stimulus-utils";

// eslint-disable-next-line @typescript-eslint/unbound-method -- This is fine.
const { targets, getTarget } = defineTargets({
  editButton: "button",
  submit: "button",
  view: "p",
  editForm: "form",
  editTextarea: "textarea",
  cancelButton: "button",
});

export class EditCommentController extends Controller<HTMLDivElement> {
  public static identifier = "edit-comment";
  public static targets = targets;
  getTarget = getTarget;

  override connect(): void {
    const $submit = this.getTarget("submit");
    const $editButton = this.getTarget("editButton");
    const $editTextarea = this.getTarget("editTextarea");
    const $cancelButton = this.getTarget("cancelButton");

    $editButton.addEventListener("click", () => {
      this.toggleEditMode("edit");
    });

    $cancelButton.addEventListener("click", () => {
      this.toggleEditMode("view");
    });

    $editTextarea.addEventListener("keydown", (event) => {
      // Ctrl+Enter for submit.
      if (event.ctrlKey && event.key == "Enter") {
        $submit.click();
      }
    });
  }

  private toggleEditMode(mode: "view" | "edit"): void {
    const $view = this.getTarget("view");
    const $editForm = this.getTarget("editForm");
    const $editTextarea = this.getTarget("editTextarea");

    if (mode === "edit") {
      jsUtils.hide($view);
      jsUtils.show($editForm);
      // Do NOT update $textarea.value here.
      // It's already set in the HTML template.
      // Doing it this way means the value is preserved across refreshes, which avoids losing your
      // edit if you accidentally refresh the page.
      $editTextarea.focus();
    } else {
      jsUtils.show($view);
      jsUtils.hide($editForm);
      $editTextarea.value = $view.textContent ?? "";
      $editTextarea.focus();
    }
  }
}
