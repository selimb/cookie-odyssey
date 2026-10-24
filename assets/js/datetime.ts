import { Controller } from "@hotwired/stimulus";

function updateDate($time: HTMLTimeElement): void {
  const datetimeString = $time.getAttribute("datetime");
  if (!datetimeString) return;

  // [date-as-datetime] A date-only string is parsed as midnight UTC, which is
  // the previous day west of UTC.
  // A datetime without an offset is parsed as local time instead.
  // So dates are sent as datetimes at midnight, never as plain dates.
  const date = new Date(datetimeString);
  if (Number.isNaN(date.getTime())) {
    // eslint-disable-next-line no-console -- TODO [error-reporting]
    console.error(`Invalid datetime: '${datetimeString}'`);
    return;
  }

  const dateStyle = $time.getAttribute("data-date-style") ?? undefined;
  const timeStyle = $time.getAttribute("data-time-style") ?? undefined;
  const text = Intl.DateTimeFormat(undefined, {
    dateStyle: dateStyle as never,
    timeStyle: timeStyle as never,
  }).format(date);
  $time.textContent = text;
}

export class DatetimeController extends Controller<HTMLTimeElement> {
  public static identifier = "datetime";

  connect(): void {
    updateDate(this.element);
  }
}
