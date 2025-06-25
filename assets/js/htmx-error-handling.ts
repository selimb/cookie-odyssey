import htmx from "htmx.org";
import { toast } from "./toast";
import type { HtmxResponseErrorEvent, HtmxSendErrorEvent } from "./htmx-types";

htmx.on("htmx:responseError", (event_) => {
  const event = event_ as HtmxResponseErrorEvent;
  let error = event.detail.error + "\n" + event.detail.xhr.responseText;
  console.error("Response Error:", error);
  toast({ message: error, variant: "error" });
});

htmx.on("htmx:sendError", (event_) => {
  const event = event_ as HtmxSendErrorEvent;
  let url = event.detail.xhr.responseURL;
  let error = `Network error: ${url}`;
  console.error(error);
  toast({ message: error, variant: "error" });
});
