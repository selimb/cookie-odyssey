export type HtmxResponseErrorEvent = CustomEvent<{
  // Not documented?
  error: string;
  xhr: { responseText: string };
}>;

export type HtmxSendErrorEvent = CustomEvent<{
  xhr: { responseURL: string };
}>;

export type HtmxAfterRequestEvent = CustomEvent<{
  successful: boolean;
}>;
