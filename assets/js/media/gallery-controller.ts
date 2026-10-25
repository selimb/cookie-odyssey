import { TypedController } from "../utils/stimulus-typed";
import type { MediaType } from "./form-controller";
import {
  clampScale,
  clampTransform,
  fitSize,
  IDENTITY,
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

/**
 * Delay, in ms, before a single mouse click is acted upon, since it may still
 * turn out to be the first click of a double-click.
 */
const DOUBLE_CLICK_MS = 300;
/** Scale used by double-click. */
const DOUBLE_CLICK_SCALE = 2.5;

/**
 * Full-screen viewer for the images and videos of a page.
 *
 * Swiping is native scroll snapping.
 * [gallery-native-zoom] On touch screens, zoom is the browser's own pinch
 * zoom of the visual viewport, which feels much better than anything driven by
 * touch events.
 * The viewer only locks swiping and hides its chrome while the page is zoomed.
 * With a mouse, zoom (images only) is a CSS transform, see [gallery-zoom].
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

  /** Type of the pointer that started the latest click. */
  private pointerType = "";
  private singleClickTimer: number | undefined;
  private mouseDown: Point | undefined;

  connect(): void {
    this.abort = new AbortController();
    const { signal } = this.abort;
    const scroller = this.getTarget("scroller");

    scroller.addEventListener("pointerdown", this.onPointerDown, { signal });
    scroller.addEventListener("dblclick", this.onDoubleClick, { signal });
    scroller.addEventListener("click", this.onClick, { signal });
    scroller.addEventListener("mousedown", this.onMouseDown, { signal });
    window.addEventListener("mousemove", this.onMouseMove, { signal });
    window.addEventListener("mouseup", this.onMouseUp, { signal });
    // See [gallery-native-zoom].
    window.visualViewport?.addEventListener("resize", this.onViewportResize, {
      signal,
    });
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
    this.onViewportResize();
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
    this.cancelSingleClick();
    this.mouseDown = undefined;
    this.cancelZoomFrame();
    for (const slide of this.slides) {
      if (slide.media instanceof HTMLVideoElement) slide.media.pause();
    }
    this.isOpen = false;
    this.getTarget("dialog").classList.remove(
      "is-zoomed",
      "is-viewport-zoomed",
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

  /** Tracks the browser's pinch zoom, see [gallery-native-zoom]. */
  private onViewportResize = (): void => {
    if (!this.isOpen) return;
    const scale = window.visualViewport?.scale ?? 1;
    this.getTarget("dialog").classList.toggle(
      "is-viewport-zoomed",
      // Leaves some room for rounding.
      scale > 1.01,
    );
  };

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
      this.setZoom(zoomAt(this.zoom, DOUBLE_CLICK_SCALE, point), "animate");
    }
  }

  /** Returns whether the event targets the image of the current slide. */
  private hitsImage(event: Event): boolean {
    const image = this.currentImage();
    return image !== undefined && event.target === image;
  }

  // Click handling.

  private onPointerDown = (event: PointerEvent): void => {
    this.pointerType = event.pointerType;
  };

  private onClick = (event: MouseEvent): void => {
    if (!this.hitsImage(event)) return;
    if (this.pointerType !== "mouse") {
      // Touch screens have no double-click to wait for, see
      // [gallery-native-zoom].
      this.toggleChrome();
      return;
    }
    // Second click of a double-click is handled by `onDoubleClick`.
    if (event.detail > 1) return;
    this.cancelSingleClick();
    this.singleClickTimer = window.setTimeout(() => {
      this.singleClickTimer = undefined;
      this.toggleChrome();
    }, DOUBLE_CLICK_MS);
  };

  private cancelSingleClick(): void {
    if (this.singleClickTimer !== undefined) {
      clearTimeout(this.singleClickTimer);
      this.singleClickTimer = undefined;
    }
  }

  private toggleChrome(): void {
    const dialog = this.getTarget("dialog");
    if (!this.isOpen || this.isZoomed()) return;
    if (dialog.classList.contains("is-viewport-zoomed")) return;
    dialog.classList.toggle("chrome-hidden");
  }

  // Mouse zoom.

  private onDoubleClick = (event: MouseEvent): void => {
    if (!this.hitsImage(event) || this.pointerType !== "mouse") return;
    this.cancelSingleClick();
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
