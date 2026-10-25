import { TypedController } from "../utils/stimulus-typed";
import type { MediaType } from "./form-controller";
import {
  clampScale,
  clampTransform,
  distance,
  fitSize,
  IDENTITY,
  midpoint,
  pinch,
  type Point,
  type Transform,
  zoomAt,
} from "./zoom";

// [gallery-item-data] Data read from each item anchor and its thumbnail.
type GalleryItem = {
  type: MediaType;
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

/** How a zoom change is written to the DOM. */
type ZoomWrite = "frame" | "now" | "animate";

/** A touch that may turn out to be a tap. */
type Tap = { point: Point; time: number };

/** Maximum movement, in px, for a touch to still count as a tap. */
const TAP_SLOP = 10;
/** Maximum duration, in ms, of a touch that counts as a tap. */
const TAP_MAX_MS = 500;
/**
 * Maximum delay, in ms, between the taps of a double-tap.
 * Also the delay before a single tap is acted upon, since it may still turn
 * out to be the first tap of a double-tap.
 */
const DOUBLE_TAP_MS = 300;
/** Maximum distance, in px, between the taps of a double-tap. */
const DOUBLE_TAP_SLOP = 40;
/** Scale used by double-tap and double-click. */
const DOUBLE_TAP_SCALE = 2.5;
/** Scale below which a finished pinch snaps back to no zoom. */
const SNAP_BACK_SCALE = 1.05;

/**
 * Full-screen viewer for the images and videos of a page.
 *
 * Swiping is native scroll snapping.
 * Zoom (images only) is a CSS transform driven by touch events.
 * See [gallery-zoom].
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
  private previousHtmlOverflow = "";
  private abort: AbortController | undefined;

  private zoom: Transform = IDENTITY;
  private zoomFrame: number | undefined;
  private resizeObserver: ResizeObserver | undefined;

  // Touch gesture state.
  private pinchStart:
    | { transform: Transform; distance: number; mid: Point }
    | undefined;
  private panStart: Point | undefined;
  private tapStart: Tap | undefined;
  private lastTap: Tap | undefined;
  private singleTapTimer: number | undefined;
  private mouseDown: Point | undefined;

  connect(): void {
    this.abort = new AbortController();
    const { signal } = this.abort;
    const scroller = this.getTarget("scroller");
    const dialog = this.getTarget("dialog");

    // Touch listeners must be non-passive to be able to cancel native
    // scrolling, pinch and clicks.
    const active = { passive: false, signal };
    scroller.addEventListener("touchstart", this.onTouchStart, active);
    scroller.addEventListener("touchmove", this.onTouchMove, active);
    scroller.addEventListener("touchend", this.onTouchEnd, active);
    scroller.addEventListener("touchcancel", this.resetTouch, { signal });
    scroller.addEventListener("dblclick", this.onDoubleClick, { signal });
    scroller.addEventListener("click", this.onClick, { signal });
    scroller.addEventListener("mousedown", this.onMouseDown, { signal });
    window.addEventListener("mousemove", this.onMouseMove, { signal });
    window.addEventListener("mouseup", this.onMouseUp, { signal });
    // Stops iOS Safari from zooming the page on pinch.
    dialog.addEventListener("gesturestart", preventDefault, { signal });
    // Observes the scroller rather than the window, since its size can
    // change on its own (e.g. the dynamic viewport height on mobile), and
    // since the callback runs before the stale scroll position is reported.
    this.resizeObserver = new ResizeObserver(this.onResize);
    this.resizeObserver.observe(scroller);
  }

  disconnect(): void {
    this.abort?.abort();
    this.abort = undefined;
    this.resizeObserver?.disconnect();
    this.resizeObserver = undefined;
    if (this.isOpen) this.teardown();
  }

  /** Opens the viewer on the clicked item. */
  open(event: MouseEvent): void {
    // Lets modified clicks open the original in a new tab.
    if (event.metaKey || event.ctrlKey || event.shiftKey || event.altKey) {
      return;
    }
    const anchor = event.currentTarget as HTMLAnchorElement;
    const index = this.itemTargets.indexOf(anchor);
    if (index === -1) return;
    event.preventDefault();

    this.buildSlides();
    this.isOpen = true;
    this.index = index;

    this.previousHtmlOverflow = document.documentElement.style.overflow;
    document.documentElement.style.overflow = "hidden";

    this.getTarget("dialog").showModal();
    this.scrollToIndex(index, "instant");
    this.syncIndex();
  }

  /**
   * Closes the viewer, from the close button or when the dialog closed on its
   * own (e.g. Esc, or the Android back gesture).
   */
  close(): void {
    if (this.isOpen) this.teardown();
  }

  prev(): void {
    this.go(-1);
  }

  next(): void {
    this.go(1);
  }

  /** Handles arrow key navigation. */
  key(event: KeyboardEvent): void {
    // Leaves arrow keys to the native video controls, for seeking.
    if (event.target instanceof HTMLVideoElement) return;
    if (event.key === "ArrowLeft") {
      event.preventDefault();
      this.go(-1);
    } else if (event.key === "ArrowRight") {
      event.preventDefault();
      this.go(1);
    }
  }

  /** Handles scrolling of the slides, updating the current index. */
  scrolled(): void {
    if (!this.isOpen) return;
    this.syncIndex();
  }

  /** Closes the dialog and releases everything that open acquired. */
  private teardown(): void {
    this.cancelSingleTap();
    this.resetTouch();
    this.cancelZoomFrame();
    for (const slide of this.slides) {
      if (slide.media instanceof HTMLVideoElement) slide.media.pause();
    }
    this.isOpen = false;
    this.getTarget("dialog").classList.remove(
      "is-zoomed",
      "is-pinching",
      "chrome-hidden",
    );
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
    if (index !== this.index) {
      this.resetZoom();
      this.index = index;
      this.lastTap = undefined;
      // Taps on videos are left to the native controls, so the chrome could
      // not be brought back.
      if (this.slides[index]?.media instanceof HTMLVideoElement) {
        this.getTarget("dialog").classList.remove("chrome-hidden");
      }
    }
    for (const [i, slide] of this.slides.entries()) {
      if (i !== index && slide.media instanceof HTMLVideoElement) {
        slide.media.pause();
      }
    }
    // Only slides near the current one load their original.
    for (const slide of this.slides.slice(Math.max(0, index - 1), index + 2)) {
      loadOriginal(slide);
    }
    this.getTarget("counter").textContent =
      `${index + 1} / ${this.slides.length}`;
    this.getTarget("prev").disabled = index === 0;
    this.getTarget("next").disabled = index === this.slides.length - 1;
  }

  private onResize = (): void => {
    if (!this.isOpen || this.getTarget("scroller").clientWidth === 0) return;
    this.resetZoom();
    this.scrollToIndex(this.index, "instant");
  };

  // Zoom.

  /** Returns the image of the current slide, if it is one. */
  private currentImage(): HTMLImageElement | undefined {
    const media = this.slides.at(this.index)?.media;
    return media instanceof HTMLImageElement ? media : undefined;
  }

  /**
   * Converts client coordinates to the coordinates of the zoomed element,
   * which are those of its (untransformed) slide.
   */
  private toLocal(clientX: number, clientY: number): Point {
    const slide = this.slides.at(this.index);
    if (!slide) return { x: clientX, y: clientY };
    const rect = slide.element.getBoundingClientRect();
    return { x: clientX - rect.left, y: clientY - rect.top };
  }

  private isZoomed(): boolean {
    return this.zoom.scale > 1;
  }

  /** Sets the zoom of the current image, clamped, and writes it to the DOM. */
  private setZoom(t: Transform, write: ZoomWrite = "frame"): void {
    const slide = this.slides.at(this.index);
    const image = this.currentImage();
    if (!slide || !image) return;
    const view = {
      width: slide.element.clientWidth,
      height: slide.element.clientHeight,
    };
    // The thumbnail and the original have the same aspect ratio, and the
    // natural size is more reliable than the stored one.
    const natural =
      image.naturalWidth > 0
        ? { width: image.naturalWidth, height: image.naturalHeight }
        : { width: slide.item.width, height: slide.item.height };
    const fit = fitSize(natural, view);
    this.zoom = clampTransform({ ...t, scale: clampScale(t.scale) }, view, fit);
    this.getTarget("dialog").classList.toggle("is-zoomed", this.isZoomed());
    if (write === "frame") {
      // Coalesces the writes of a gesture to one per frame.
      this.zoomFrame ??= requestAnimationFrame(() => {
        this.zoomFrame = undefined;
        writeTransform(image, this.zoom, false);
      });
    } else {
      this.cancelZoomFrame();
      writeTransform(image, this.zoom, write === "animate");
    }
  }

  private cancelZoomFrame(): void {
    if (this.zoomFrame !== undefined) {
      cancelAnimationFrame(this.zoomFrame);
      this.zoomFrame = undefined;
    }
  }

  /** Resets the zoom of the current slide right away, before it changes. */
  private resetZoom(): void {
    if (!this.isZoomed()) return;
    this.setZoom(IDENTITY, "now");
  }

  private toggleZoomAt(point: Point): void {
    if (this.isZoomed()) {
      this.setZoom(IDENTITY, "animate");
    } else {
      this.setZoom(zoomAt(this.zoom, DOUBLE_TAP_SCALE, point), "animate");
    }
  }

  /** Returns whether the event targets the image of the current slide. */
  private hitsImage(event: Event): boolean {
    const image = this.currentImage();
    return image !== undefined && event.target === image;
  }

  // Touch handling.

  private onTouchStart = (event: TouchEvent): void => {
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
      // The first finger may have started scrolling the slides already, in
      // which case touch events can no longer cancel it.
      // Stops the scrolling and puts the current slide back in place.
      this.getTarget("dialog").classList.add("is-pinching");
      this.scrollToIndex(this.index, "instant");
      if (event.cancelable) event.preventDefault();
      return;
    }
    if (touches.length === 1) {
      const t = touches.item(0);
      if (!t) return;
      const point = this.toLocal(t.clientX, t.clientY);
      this.tapStart = { point, time: event.timeStamp };
      if (this.isZoomed()) {
        this.panStart = point;
      }
    }
  };

  private onTouchMove = (event: TouchEvent): void => {
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
      this.setZoom(panBy(this.zoom, this.panStart, point));
      // Pans incrementally, so that moving back after hitting an edge moves
      // the image right away.
      this.panStart = point;
      if (event.cancelable) event.preventDefault();
    }
  };

  private onTouchEnd = (event: TouchEvent): void => {
    if (event.touches.length === 0) {
      this.getTarget("dialog").classList.remove("is-pinching");
    }
    if (this.pinchStart) {
      if (event.touches.length < 2) {
        this.pinchStart = undefined;
        if (this.zoom.scale < SNAP_BACK_SCALE) {
          this.setZoom(IDENTITY, "animate");
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
    if (event.timeStamp - tap.time > TAP_MAX_MS) return;
    // Suppresses the synthesized mouse events and click.
    if (event.cancelable) event.preventDefault();

    const last = this.lastTap;
    if (
      last &&
      event.timeStamp - last.time < DOUBLE_TAP_MS &&
      distance(last.point, tap.point) < DOUBLE_TAP_SLOP
    ) {
      this.lastTap = undefined;
      this.cancelSingleTap();
      this.toggleZoomAt(tap.point);
      return;
    }
    this.lastTap = { point: tap.point, time: event.timeStamp };
    this.scheduleSingleTap();
  };

  private resetTouch = (): void => {
    this.getTarget("dialog").classList.remove("is-pinching");
    this.pinchStart = undefined;
    this.panStart = undefined;
    this.tapStart = undefined;
    this.mouseDown = undefined;
  };

  private scheduleSingleTap(): void {
    this.cancelSingleTap();
    this.singleTapTimer = window.setTimeout(() => {
      this.singleTapTimer = undefined;
      this.onSingleTap();
    }, DOUBLE_TAP_MS);
  }

  private cancelSingleTap(): void {
    if (this.singleTapTimer !== undefined) {
      clearTimeout(this.singleTapTimer);
      this.singleTapTimer = undefined;
    }
  }

  private onSingleTap(): void {
    if (!this.isOpen || this.isZoomed()) return;
    this.getTarget("dialog").classList.toggle("chrome-hidden");
  }

  // Mouse handling.

  private onClick = (event: MouseEvent): void => {
    if (!this.hitsImage(event)) return;
    // Second click of a double-click is handled by `onDoubleClick`.
    if (event.detail > 1) return;
    this.scheduleSingleTap();
  };

  private onDoubleClick = (event: MouseEvent): void => {
    if (!this.hitsImage(event)) return;
    this.cancelSingleTap();
    this.toggleZoomAt(this.toLocal(event.clientX, event.clientY));
  };

  private onMouseDown = (event: MouseEvent): void => {
    if (!this.hitsImage(event) || !this.isZoomed()) return;
    event.preventDefault();
    this.mouseDown = this.toLocal(event.clientX, event.clientY);
  };

  private onMouseMove = (event: MouseEvent): void => {
    const start = this.mouseDown;
    if (!start) return;
    const point = this.toLocal(event.clientX, event.clientY);
    this.setZoom(panBy(this.zoom, start, point));
    this.mouseDown = point;
  };

  private onMouseUp = (): void => {
    this.mouseDown = undefined;
  };
}

/** Returns `t` translated by the movement from `from` to `to`. */
function panBy(t: Transform, from: Point, to: Point): Transform {
  return { scale: t.scale, x: t.x + to.x - from.x, y: t.y + to.y - from.y };
}

function preventDefault(event: Event): void {
  event.preventDefault();
}

function writeTransform(
  image: HTMLElement,
  t: Transform,
  animate: boolean,
): void {
  image.classList.toggle("gallery-animate", animate);
  image.style.transform =
    t.scale === 1 && t.x === 0 && t.y === 0
      ? ""
      : `translate(${t.x}px, ${t.y}px) scale(${t.scale})`;
}

/** Reads an item anchor, see [gallery-item-data]. */
function readItem(anchor: HTMLAnchorElement): GalleryItem {
  const d = anchor.dataset;
  const thumbnail = anchor.querySelector("img");
  return {
    type: d.mediaType === "image" ? "image" : "video",
    urlOriginal: anchor.href,
    urlThumbnail: thumbnail?.src ?? "",
    width: Number(d.width),
    height: Number(d.height),
    caption: thumbnail?.alt ?? "",
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

/** Starts loading the original of a slide, once per opening of the viewer. */
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
      // Keeps showing the thumbnail, rather than retrying on every scroll.
    });
}
