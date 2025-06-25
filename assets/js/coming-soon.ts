import { Controller } from "@hotwired/stimulus";

import { jsUtils } from "./js-utils";
import { toast } from "./toast";

const MESSAGES = [
  "Coming soon!",
  "I said coming soon!",
  "Seriously?",
  "Press it again, I dare you",
  "I double dare you",
];

export class ComingSoonController extends Controller<HTMLButtonElement> {
  public static identifier = "coming-soon";

  connect(): void {
    let counter = 0;

    this.element.addEventListener("click", () => {
      const msg = MESSAGES.at(counter++);

      if (msg == null) {
        jsUtils.hide(this.element);
      } else {
        toast({ message: msg, variant: "success", auto_close: true });
      }
    });
  }
}
