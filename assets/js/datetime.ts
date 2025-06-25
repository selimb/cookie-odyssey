import { Controller } from "@hotwired/stimulus";

function updateDate($time: HTMLTimeElement): void {
  const datetimeString = $time.getAttribute("datetime");
  if (!datetimeString) return;

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
