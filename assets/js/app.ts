import * as Stimulus from "@hotwired/stimulus";
import htmx from "htmx.org";
import "htmx-ext-response-targets";
import { DatetimeController } from "./datetime";
import { toast, ToastController } from "./toast";
import { fireConfetti } from "./confetti";
import "./htmx-error-handling";
import { ThemeToggleController } from "./theme";
import { ComingSoonController } from "./coming-soon";
import { JournalEntryMediaFormController } from "./media/form-controller";
import { AddCommentController } from "./comment/add-comment";
import { EditCommentController } from "./comment/edit-comment";

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
  interface Window {
    htmx: typeof htmx;
    Stimulus: Stimulus.Application;
    toast: typeof toast;
    // [confetti-fn]
    fireConfetti: typeof fireConfetti;
  }
}
