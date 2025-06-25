import "htmx-ext-response-targets";
import "./htmx-error-handling";

import * as Stimulus from "@hotwired/stimulus";
import htmx from "htmx.org";

import { ComingSoonController } from "./coming-soon";
import { AddCommentController } from "./comment/add-comment";
import { EditCommentController } from "./comment/edit-comment";
import { fireConfetti } from "./confetti";
import { DatetimeController } from "./datetime";
import { JournalEntryMediaFormController } from "./media/form-controller";
import { ThemeToggleController } from "./theme";
import { toast, ToastController } from "./toast";

const stimulus = Stimulus.Application.start();

// stimulus.debug = true;

for (const controller of [
  DatetimeController,
  ToastController,
  ThemeToggleController,
  ComingSoonController,
  JournalEntryMediaFormController,
  AddCommentController,
  EditCommentController,
]) {
  stimulus.register(controller.identifier, controller);
}

// Lets you do `Stimulus.debug = true` in Devtools console.
window.Stimulus = stimulus;

// Required to easily use `htmx` from inline script.
window.htmx = htmx;

window.toast = toast;

window.fireConfetti = fireConfetti;

declare global {
  // eslint-disable-next-line @typescript-eslint/consistent-type-definitions -- Need interface augmentation
  interface Window {
    htmx: typeof htmx;
    Stimulus: Stimulus.Application;
    toast: typeof toast;
    // [confetti-fn]
    fireConfetti: typeof fireConfetti;
  }
}
