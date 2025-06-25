import { Controller } from "@hotwired/stimulus";

function updateDate($time: HTMLTimeElement): void {
  const datetimeString = $time.getAttribute("datetime");
  if (!datetimeString) return;

  const date = new Date(datetimeString);
  if (isNaN(date.getTime())) {
    console.error(`Invalid datetime: '${datetimeString}'`);
    return;
  }

  const dateStyle = $time.getAttribute("data-date-style") ?? undefined;
  const timeStyle = $time.getAttribute("data-time-style") ?? undefined;
  const text = Intl.DateTimeFormat(undefined, {
    dateStyle: dateStyle as any,
    timeStyle: timeStyle as any,
  }).format(date);
  $time.textContent = text;
}

export class DatetimeController extends Controller<HTMLTimeElement> {
  public static identifier = "datetime";

  connect(): void {
    updateDate(this.element);
  }
}
