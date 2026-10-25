import { TypedController } from "../utils/stimulus-typed";
import {
  clampScale,
  clampTransform,
  distance,
  DOUBLE_TAP_SCALE,
  fitSize,
  IDENTITY,
  midpoint,
  pinch,
  type Point,
  SNAP_BACK_SCALE,
  type Transform,
  zoomAt,
} from "./zoom";

// [gallery-item-data] Data read from each item anchor, rendered by
// `gallery_item` in `templates/common/media.html`.
type GalleryItem = {
  type: "image" | "video";
  urlOriginal: string;
  urlThumbnail: string;
  width: number;
  height: number;
  caption: string;
};

type Slide = {
  item: GalleryItem;
  element: HTMLElement;
  media: HTMLImageElement | HTMLVideoElement;
  /** Whether the original has been requested. */
  loaded: boolean;
};

/** Maximum movement, in px, for a touch to still count as a tap. */
const TAP_SLOP = 10;
/** Maximum delay, in ms, between the taps of a double-tap. */
const DOUBLE_TAP_MS = 300;
/** Delay, in ms, before a single tap is acted upon. */
const SINGLE_TAP_DELAY_MS = 250;

/**
 * Full-screen viewer for the images and videos of a page.
 *
 * Swiping is native scroll snapping.
 * Zoom (images only) is a CSS transform driven by touch events, see
 * `./zoom.ts` for the math.
 */
export class JournalEntryMediaGalleryController extends TypedController(
  "media--gallery",
  "element",
  {
    targets: {
      item: "a",
      dialog: "dialog",
      scroller: "div",
      counter: "span",
      prev: "button",
      next: "button",
    },
  },
) {
  declare readonly itemTargets: HTMLAnchorElement[];

  private slides: Slide[] = [];
  private index = 0;
  private isOpen = false;
  // Whether we pushed a history entry that has not been popped yet.
  private hasHistoryEntry = false;
  // Whether `history.back()` was requested and its popstate is pending.
  private backPending = false;
  private previousHtmlOverflow = "";
  private abort: AbortController | undefined;

  private chromeHidden = false;
  private zoom: Transform = IDENTITY;
  private zoomFrame: number | undefined;

  // Touch gesture state.
  private pinchStart:
    | { transform: Transform; distance: number; mid: Point }
    | undefined;
  private panStart: { touch: Point; transform: Transform } | undefined;
  private tapStart: { point: Point; time: number } | undefined;
  private lastTap: { point: Point; time: number } | undefined;
  private singleTapTimer: number | undefined;
  private mouseDown: { point: Point; transform: Transform } | undefined;

  connect(): void {
    this.abort = new AbortController();
    const { signal } = this.abort;
    const scroller = this.getTarget("scroller");
    const dialog = this.getTarget("dialog");

    // Touch listeners must be non-passive to be able to block native pinch.
    scroller.addEventListener(
      "touchstart",
      (e) => {
        this.onTouchStart(e);
      },
      {
        passive: false,
        signal,
      },
    );
    scroller.addEventListener(
      "touchmove",
      (e) => {
        this.onTouchMove(e);
      },
      {
        passive: false,
        signal,
      },
    );
    scroller.addEventListener(
      "touchend",
      (e) => {
        this.onTouchEnd(e);
      },
      {
        passive: false,
        signal,
      },
    );
    scroller.addEventListener(
      "touchcancel",
      () => {
        this.resetTouch();
      },
      {
        signal,
      },
    );
    scroller.addEventListener(
      "dblclick",
      (e) => {
        this.onDoubleClick(e);
      },
      {
        signal,
      },
    );
    scroller.addEventListener(
      "click",
      (e) => {
        this.onClick(e);
      },
      { signal },
    );
    scroller.addEventListener(
      "mousedown",
      (e) => {
        this.onMouseDown(e);
      },
      {
        signal,
      },
    );
    window.addEventListener(
      "mousemove",
      (e) => {
        this.onMouseMove(e);
      },
      {
        signal,
      },
    );
    window.addEventListener("mouseup", () => (this.mouseDown = undefined), {
      signal,
    });
    // Stops iOS Safari from zooming the page on pinch.
    dialog.addEventListener(
      "gesturestart",
      (e) => {
        e.preventDefault();
      },
      {
        signal,
      },
    );
    window.addEventListener(
      "resize",
      () => {
        this.onResize();
      },
      { signal },
    );
  }

  disconnect(): void {
    this.abort?.abort();
    this.abort = undefined;
    if (this.isOpen) {
      // The element is going away, so there is no point in popping history.
      this.teardown();
    }
  }

  /** Opens the viewer on the clicked item. */
  open(event: Event): void {
    const anchor = event.currentTarget as HTMLAnchorElement;
    const index = this.itemTargets.indexOf(anchor);
    if (index === -1 || this.isOpen) return;
    event.preventDefault();

    this.buildSlides();
    this.isOpen = true;
    this.index = index;
    this.chromeHidden = false;
    this.applyChromeClasses();

    this.previousHtmlOverflow = document.documentElement.style.overflow;
    document.documentElement.style.overflow = "hidden";

    // Pushed so that the back button closes the viewer. See [gallery-popstate].
    history.pushState({ gallery: true }, "", location.href);
    this.hasHistoryEntry = true;
    this.backPending = false;
    window.addEventListener("popstate", this.onPopState, {
      capture: true,
      signal: this.abort?.signal,
    });

    this.getTarget("dialog").showModal();
    this.scrollToIndex(index, "instant");
    this.syncIndex();
  }

  /** Requests closing the viewer, through history. */
  close(): void {
    this.requestBack();
  }

  /** Handles the native dialog cancel (Esc), going through history. */
  cancel(event: Event): void {
    event.preventDefault();
    this.requestBack();
  }

  prev(): void {
    this.go(-1);
  }

  next(): void {
    this.go(1);
  }

  /** Handles arrow key navigation. */
  key(event: KeyboardEvent): void {
    if (event.key === "ArrowLeft") {
      this.go(-1);
    } else if (event.key === "ArrowRight") {
      this.go(1);
    }
  }

  /** Handles scrolling of the slides, updating the current index. */
  scrolled(): void {
    if (!this.isOpen) return;
    this.syncIndex();
  }

  // [gallery-popstate] htmx listens to `popstate` on `window` and restores
  // its history cache when the state we pop back to has `htmx: true`, which is
  // the case for the page that opened the viewer.
  // Our listener is registered in the capture phase, and stops the event.
  private onPopState = (event: PopStateEvent): void => {
    if (!this.isOpen) return;
    event.stopImmediatePropagation();
    this.hasHistoryEntry = false;
    this.backPending = false;
    this.teardown();
  };

  private requestBack(): void {
    if (!this.isOpen || this.backPending) return;
    if (this.hasHistoryEntry) {
      this.backPending = true;
      history.back();
    } else {
      this.teardown();
    }
  }

  /** Closes the dialog and releases everything that open acquired. */
  private teardown(): void {
    window.removeEventListener("popstate", this.onPopState, { capture: true });
    this.cancelSingleTap();
    this.resetTouch();
    if (this.zoomFrame !== undefined) {
      cancelAnimationFrame(this.zoomFrame);
      this.zoomFrame = undefined;
    }
    for (const slide of this.slides) {
      if (slide.media instanceof HTMLVideoElement) slide.media.pause();
    }
    this.isOpen = false;
    this.getTarget("dialog").classList.remove("is-zoomed", "chrome-hidden");
    this.getTarget("dialog").close();
    this.getTarget("scroller").replaceChildren();
    this.slides = [];
    this.zoom = IDENTITY;
    document.documentElement.style.overflow = this.previousHtmlOverflow;
  }

  private buildSlides(): void {
    const scroller = this.getTarget("scroller");
    this.slides = this.itemTargets.map((anchor) => {
      const item = readItem(anchor);
      const slide = createSlide(item);
      scroller.append(slide.element);
      return slide;
    });
  }

  private go(delta: number): void {
    const target = this.index + delta;
    if (target < 0 || target >= this.slides.length) return;
    this.scrollToIndex(target, "smooth");
  }

  private scrollToIndex(index: number, behavior: ScrollBehavior): void {
    const scroller = this.getTarget("scroller");
    scroller.scrollTo({ left: index * scroller.clientWidth, behavior });
  }

  /** Derives the current index from the scroll position and reacts to it. */
  private syncIndex(): void {
    const scroller = this.getTarget("scroller");
    const width = scroller.clientWidth;
    if (width === 0) return;
    const index = Math.min(
      this.slides.length - 1,
      Math.max(0, Math.round(scroller.scrollLeft / width)),
    );
    const changed = index !== this.index;
    if (changed) {
      this.resetZoom(false);
      this.index = index;
    }
    for (const [i, slide] of this.slides.entries()) {
      if (i !== index && slide.media instanceof HTMLVideoElement) {
        slide.media.pause();
      }
    }
    // Only slides near the current one load their original.
    for (let i = index - 1; i <= index + 1; i++) {
      const slide = this.slides.at(i);
      if (slide) loadOriginal(slide);
    }
    this.getTarget("counter").textContent =
      `${index + 1} / ${this.slides.length}`;
    this.getTarget("prev").disabled = index === 0;
    this.getTarget("next").disabled = index === this.slides.length - 1;
  }

  private onResize(): void {
    if (!this.isOpen) return;
    this.resetZoom(false);
    this.scrollToIndex(this.index, "instant");
  }

  // Chrome.

  private toggleChrome(): void {
    this.chromeHidden = !this.chromeHidden;
    this.applyChromeClasses();
  }

  private applyChromeClasses(): void {
    this.getTarget("dialog").classList.toggle(
      "chrome-hidden",
      this.chromeHidden,
    );
  }

  // Zoom.

  /** Returns the current slide if it is an image, otherwise undefined. */
  private currentImage(): HTMLImageElement | undefined {
    const media = this.slides.at(this.index)?.media;
    return media instanceof HTMLImageElement ? media : undefined;
  }

  private viewSize(): { width: number; height: number } {
    const scroller = this.getTarget("scroller");
    return { width: scroller.clientWidth, height: scroller.clientHeight };
  }

  /** Converts client coordinates to the coordinates of the zoomed element. */
  private toLocal(clientX: number, clientY: number): Point {
    const rect = this.getTarget("scroller").getBoundingClientRect();
    return { x: clientX - rect.left, y: clientY - rect.top };
  }

  private isZoomed(): boolean {
    return this.zoom.scale > 1;
  }

  /** Sets the zoom, clamped, writing it to the DOM on the next frame. */
  private setZoom(t: Transform, animate = false): void {
    const slide = this.slides.at(this.index);
    const image = this.currentImage();
    if (!slide || !image) return;
    const view = this.viewSize();
    const fit = fitSize(
      { width: slide.item.width, height: slide.item.height },
      view,
    );
    this.zoom = clampTransform({ ...t, scale: clampScale(t.scale) }, view, fit);
    this.getTarget("dialog").classList.toggle("is-zoomed", this.isZoomed());
    if (animate) {
      if (this.zoomFrame !== undefined) cancelAnimationFrame(this.zoomFrame);
      this.zoomFrame = undefined;
      image.classList.add("gallery-animate");
      writeTransform(image, this.zoom);
    } else {
      this.zoomFrame ??= requestAnimationFrame(() => {
        this.zoomFrame = undefined;
        image.classList.remove("gallery-animate");
        writeTransform(image, this.zoom);
      });
    }
  }

  /** Resets zoom of the current slide, before `index` is changed. */
  private resetZoom(animate: boolean): void {
    if (!this.isZoomed()) return;
    this.setZoom(IDENTITY, animate);
    // Not animated resets must be written right away, as the slide changes.
    if (!animate && this.zoomFrame !== undefined) {
      cancelAnimationFrame(this.zoomFrame);
      this.zoomFrame = undefined;
      const image = this.currentImage();
      if (image) writeTransform(image, this.zoom);
    }
  }

  private toggleZoomAt(point: Point): void {
    if (this.isZoomed()) {
      this.setZoom(IDENTITY, true);
    } else {
      this.setZoom(zoomAt(this.zoom, DOUBLE_TAP_SCALE, point), true);
    }
  }

  /** Whether the event targets the image of the current slide. */
  private hitsImage(event: Event): boolean {
    const image = this.currentImage();
    return image !== undefined && event.target === image;
  }

  // Touch handling.

  private onTouchStart(event: TouchEvent): void {
    if (!this.hitsImage(event) && !this.pinchStart) return;
    const touches = event.touches;
    if (touches.length === 2) {
      const [a, b] = [touches.item(0), touches.item(1)];
      if (!a || !b) return;
      const pa = this.toLocal(a.clientX, a.clientY);
      const pb = this.toLocal(b.clientX, b.clientY);
      this.pinchStart = {
        transform: this.zoom,
        distance: Math.max(1, distance(pa, pb)),
        mid: midpoint(pa, pb),
      };
      this.panStart = undefined;
      this.tapStart = undefined;
      this.cancelSingleTap();
      this.lastTap = undefined;
      event.preventDefault();
      return;
    }
    if (touches.length === 1) {
      const t = touches.item(0);
      if (!t) return;
      const point = this.toLocal(t.clientX, t.clientY);
      this.tapStart = { point, time: event.timeStamp };
      if (this.isZoomed()) {
        this.panStart = { touch: point, transform: this.zoom };
      }
    }
  }

  private onTouchMove(event: TouchEvent): void {
    const touches = event.touches;
    if (this.pinchStart && touches.length >= 2) {
      const [a, b] = [touches.item(0), touches.item(1)];
      if (!a || !b) return;
      const pa = this.toLocal(a.clientX, a.clientY);
      const pb = this.toLocal(b.clientX, b.clientY);
      this.setZoom(
        pinch(
          this.pinchStart.transform,
          this.pinchStart.distance,
          this.pinchStart.mid,
          distance(pa, pb),
          midpoint(pa, pb),
        ),
      );
      event.preventDefault();
      return;
    }
    const t = touches.item(0);
    if (!t) return;
    const point = this.toLocal(t.clientX, t.clientY);
    if (this.tapStart && distance(point, this.tapStart.point) > TAP_SLOP) {
      this.tapStart = undefined;
    }
    if (this.panStart && touches.length === 1) {
      this.setZoom({
        scale: this.panStart.transform.scale,
        x: this.panStart.transform.x + point.x - this.panStart.touch.x,
        y: this.panStart.transform.y + point.y - this.panStart.touch.y,
      });
      event.preventDefault();
    }
  }

  private onTouchEnd(event: TouchEvent): void {
    if (this.pinchStart) {
      if (event.touches.length < 2) {
        this.pinchStart = undefined;
        if (this.zoom.scale < SNAP_BACK_SCALE) {
          this.setZoom(IDENTITY, true);
        }
        // Prevents the remaining finger from panning or tapping.
        this.panStart = undefined;
        this.tapStart = undefined;
      }
      if (event.cancelable) event.preventDefault();
      return;
    }
    this.panStart = undefined;
    const tap = this.tapStart;
    this.tapStart = undefined;
    if (!tap || event.touches.length > 0) return;
    if (event.timeStamp - tap.time > 500) return;
    // Suppresses the synthesized mouse events and click.
    if (event.cancelable) event.preventDefault();

    const last = this.lastTap;
    if (
      last &&
      event.timeStamp - last.time < DOUBLE_TAP_MS &&
      distance(last.point, tap.point) < 40
    ) {
      this.lastTap = undefined;
      this.cancelSingleTap();
      this.toggleZoomAt(tap.point);
      return;
    }
    this.lastTap = { point: tap.point, time: event.timeStamp };
    this.scheduleSingleTap();
  }

  private resetTouch(): void {
    this.pinchStart = undefined;
    this.panStart = undefined;
    this.tapStart = undefined;
    this.mouseDown = undefined;
  }

  private scheduleSingleTap(): void {
    this.cancelSingleTap();
    this.singleTapTimer = window.setTimeout(() => {
      this.singleTapTimer = undefined;
      this.onSingleTap();
    }, SINGLE_TAP_DELAY_MS);
  }

  private cancelSingleTap(): void {
    if (this.singleTapTimer !== undefined) {
      clearTimeout(this.singleTapTimer);
      this.singleTapTimer = undefined;
    }
  }

  private onSingleTap(): void {
    if (!this.isOpen || this.isZoomed()) return;
    this.toggleChrome();
  }

  // Mouse handling.

  private onClick(event: MouseEvent): void {
    if (!this.hitsImage(event)) return;
    // Second click of a double-click is handled by `onDoubleClick`.
    if (event.detail > 1) return;
    this.scheduleSingleTap();
  }

  private onDoubleClick(event: MouseEvent): void {
    if (!this.hitsImage(event)) return;
    this.cancelSingleTap();
    this.toggleZoomAt(this.toLocal(event.clientX, event.clientY));
  }

  private onMouseDown(event: MouseEvent): void {
    if (!this.hitsImage(event) || !this.isZoomed()) return;
    event.preventDefault();
    this.mouseDown = {
      point: this.toLocal(event.clientX, event.clientY),
      transform: this.zoom,
    };
  }

  private onMouseMove(event: MouseEvent): void {
    const start = this.mouseDown;
    if (!start) return;
    const point = this.toLocal(event.clientX, event.clientY);
    this.setZoom({
      scale: start.transform.scale,
      x: start.transform.x + point.x - start.point.x,
      y: start.transform.y + point.y - start.point.y,
    });
  }
}

function writeTransform(image: HTMLElement, t: Transform): void {
  image.style.transform =
    t.scale === 1 && t.x === 0 && t.y === 0
      ? ""
      : `translate(${t.x}px, ${t.y}px) scale(${t.scale})`;
}

/** Reads an item anchor, see [gallery-item-data]. */
function readItem(anchor: HTMLAnchorElement): GalleryItem {
  const d = anchor.dataset;
  const type = d.mediaType === "image" ? "image" : "video";
  return {
    type,
    urlOriginal: d.urlOriginal ?? "",
    urlThumbnail: d.urlThumbnail ?? "",
    width: Number(d.width),
    height: Number(d.height),
    caption: d.caption ?? "",
  };
}

/** Creates a slide, showing the thumbnail until the original is loaded. */
function createSlide(item: GalleryItem): Slide {
  const element = document.createElement("div");
  element.className = "gallery-slide";

  let media: HTMLImageElement | HTMLVideoElement;
  if (item.type === "image") {
    const img = document.createElement("img");
    img.src = item.urlThumbnail;
    img.alt = item.caption;
    img.draggable = false;
    media = img;
  } else {
    const video = document.createElement("video");
    video.playsInline = true;
    video.controls = true;
    video.preload = "none";
    video.poster = item.urlThumbnail;
    media = video;
  }
  media.classList.add("gallery-media");
  element.append(media);

  if (item.caption) {
    const caption = document.createElement("div");
    caption.className = "gallery-caption gallery-chrome";
    if (item.type === "video") caption.classList.add("gallery-caption-video");
    caption.textContent = item.caption;
    element.append(caption);
  }
  return { item, element, media, loaded: false };
}

/** Starts loading the original of a slide, once. */
function loadOriginal(slide: Slide): void {
  if (slide.loaded) return;
  slide.loaded = true;
  const { media, item } = slide;
  if (media instanceof HTMLVideoElement) {
    media.preload = "metadata";
    media.src = item.urlOriginal;
    return;
  }
  const full = new Image();
  full.src = item.urlOriginal;
  full
    .decode()
    .then(() => {
      media.src = item.urlOriginal;
    })
    .catch(() => {
      // Keep showing the thumbnail.
      slide.loaded = false;
    });
}
