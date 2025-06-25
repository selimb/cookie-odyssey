import htmx from "htmx.org";

import type { HtmxResponseErrorEvent, HtmxSendErrorEvent } from "./htmx-types";
import { toast } from "./toast";

htmx.on("htmx:responseError", (event_) => {
  const event = event_ as HtmxResponseErrorEvent;
  const error = event.detail.error + "\n" + event.detail.xhr.responseText;
  // eslint-disable-next-line no-console -- TODO [error-reporting]
  console.error("Response Error:", error);
  toast({ message: error, variant: "error" });
});

htmx.on("htmx:sendError", (event_) => {
  const event = event_ as HtmxSendErrorEvent;
  const url = event.detail.xhr.responseURL;
  const error = `Network error: ${url}`;
  // eslint-disable-next-line no-console -- TODO [error-reporting]
  console.error(error);
  toast({ message: error, variant: "error" });
});
