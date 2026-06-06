import { z } from "zod";

import { toast } from "../toast";
import { TypedController } from "../utils/stimulus-typed";
import {
  THUMBNAIL_EXT,
  type ThumbnailFromAnyResult,
  thumbnailFromImage,
  thumbnailFromVideo,
} from "./thumbnail";

// How long the submit gate waits for in-flight uploads before giving up and
// letting the user retry. Uploads are not aborted -- they keep running.
const UPLOAD_SETTLE_TIMEOUT_MS = 30_000;

// SYNC MediaType
type MediaType = "image" | "video";

// SYNC MediaUploadUrlBody
type MediaUploadUrlBody = {
  thumbnail_extension: string;
  filenames: string[];
};

// SYNC MediaUploadUrlResultItem
const mediaUploadUrlResultItemSchema = z.object({
  upload_method: z.string(),
  upload_url_original: z.string(),
  upload_url_thumbnail: z.string(),
  upload_headers_original: z.record(z.string(), z.string()),
  upload_headers_thumbnail: z.record(z.string(), z.string()),
  file_id_original: z.number(),
  file_id_thumbnail: z.number(),
});
type MediaUploadUrlResultItem = z.infer<typeof mediaUploadUrlResultItemSchema>;

// SYNC MediaEditorItem
const mediaEditorItemSchema = z.object({
  media_type: z.enum(["image", "video"]),
  caption: z.string(),
  file_id_original: z.number(),
  width_original: z.number(),
  height_original: z.number(),
  file_id_thumbnail: z.number(),
  width_thumbnail: z.number(),
  height_thumbnail: z.number(),
  url_thumbnail: z.string(),
});
type MediaEditorItem = z.infer<typeof mediaEditorItemSchema>;

// The editor's in-memory model of a single Media item. Richer than the wire
// `MediaEditorItem`: it tracks the client-side `uid`, upload `status`, and a
// renderable `thumbnailUrl` (a local object URL for freshly-added files, a
// signed URL for items seeded from the server).
type EditorItem = {
  uid: number;
  mediaType: MediaType;
  caption: string;
  fileIdOriginal: number;
  widthOriginal: number;
  heightOriginal: number;
  fileIdThumbnail: number;
  widthThumbnail: number;
  heightThumbnail: number;
  thumbnailUrl: string;
  status: "pending" | "ready" | "error";
};

// References to the rendered DOM nodes for one item.
type ItemElements = {
  root: HTMLElement;
  thumb: HTMLImageElement;
  videoIndicator: HTMLElement;
  spinner: HTMLElement;
  caption: HTMLInputElement;
  up: HTMLButtonElement;
  down: HTMLButtonElement;
  del: HTMLButtonElement;
};

// htmx fires this before issuing a request; `issueRequest` releases a request
// that was held back with `preventDefault()`. `elt` is the element making the
// request (the form, for a form submit).
type HtmxConfirmEvent = CustomEvent<{
  elt: Element;
  issueRequest: (skipConfirmation?: boolean) => void;
}>;

type HtmxAfterRequestEvent = CustomEvent<{
  elt: Element;
  successful: boolean;
}>;

/**
 * A single client-owned Media editor, used identically on the new-entry and
 * edit pages. It owns an in-memory ordered list of Media and renders the
 * Thumbnails itself.
 *
 * Everything is staged in memory: adding a file uploads its blob to storage in
 * the background, but reorder/caption/delete only touch the in-memory list,
 * which is serialized into a hidden form field. The Media is persisted only when
 * the enclosing form is submitted (Create or Save), which the server reconciles
 * in one transaction. Two cross-cutting concerns:
 * - A `htmx:confirm` gate makes the submit wait for in-flight uploads to land.
 * - A `beforeunload` guard warns before discarding unsaved changes.
 */
export class MediaEditorController extends TypedController(
  "media--editor",
  "element",
  {
    targets: {
      list: "div",
      itemTemplate: "template",
      fileInput: "input",
      addButton: "button",
      hiddenField: "input",
    },
    values: {
      hrefUploadUrl: "string",
    },
  },
) {
  private items: EditorItem[] = [];
  private elements = new Map<number, ItemElements>();
  private uidCounter = 0;
  // One promise per added batch, covering its whole pipeline (URL fetch +
  // uploads). The submit gate awaits these so it can wait out in-flight work.
  private batches: Array<Promise<void>> = [];
  // True once the user has made changes not yet persisted by a form submit.
  private dirty = false;
  private form: HTMLFormElement | null = null;

  connect(): void {
    this.form = this.element.closest("form");

    // The hidden field is the single source of truth for the list. It is
    // server-seeded with the Entry's current Media, so it already holds the
    // correct state before this controller runs.
    const initialItems: MediaEditorItem[] = z
      .array(mediaEditorItemSchema)
      .parse(JSON.parse(this.getTarget("hiddenField").value));
    for (const wire of initialItems) {
      const item = this.itemFromWire(wire);
      this.items.push(item);
      this.renderItem(item);
    }
    this.refreshControls();
    this.syncHiddenField();

    const $addButton = this.getTarget("addButton");
    const $fileInput = this.getTarget("fileInput");

    $addButton.addEventListener("click", (event) => {
      event.preventDefault();
      $fileInput.click();
    });

    $fileInput.addEventListener("change", () => {
      if (!$fileInput.files || $fileInput.files.length === 0) {
        return;
      }
      const files = [...$fileInput.files];
      // Reset so re-selecting the same file fires `change` again.
      $fileInput.value = "";
      this.addFiles(files);
    });

    // Any edit -- to the prose fields or a caption -- marks the form dirty.
    this.form?.addEventListener("input", this.markDirty);
    this.form?.addEventListener("htmx:confirm", this.onConfirm);
    this.form?.addEventListener("htmx:after-request", this.onAfterRequest);
    window.addEventListener("beforeunload", this.onBeforeUnload);
  }

  disconnect(): void {
    this.form?.removeEventListener("input", this.markDirty);
    this.form?.removeEventListener("htmx:confirm", this.onConfirm);
    this.form?.removeEventListener("htmx:after-request", this.onAfterRequest);
    window.removeEventListener("beforeunload", this.onBeforeUnload);
    for (const item of this.items) {
      this.revokeThumbnail(item);
    }
  }

  // --- Adding files --------------------------------------------------------

  private addFiles(files: File[]): void {
    const items = files.map((file) => this.createPendingItem(file));
    for (const item of items) {
      this.items.push(item);
      this.renderItem(item);
    }
    this.dirty = true;
    this.refreshControls();
    this.syncHiddenField();

    // Run the batch in the background; the submit gate tracks it via `batches`.
    // Drop it once it settles so the array only ever holds in-flight work.
    const batch = this.runBatch(items, files);
    this.batches.push(batch);
    void batch.finally(() => {
      this.batches = this.batches.filter((b) => b !== batch);
    });
  }

  // Uploads a batch of files: generates Thumbnails, mints upload URLs, then
  // uploads each blob. Never rejects -- per-item failure is recorded as status.
  private async runBatch(items: EditorItem[], files: File[]): Promise<void> {
    const thumbnailPromises = files.map(
      async (file, index) => await this.generateThumbnail(items[index], file),
    );

    let uploadParamsList: MediaUploadUrlResultItem[];
    try {
      uploadParamsList = await fetchUploadUrls(
        files,
        this.getValue("hrefUploadUrl"),
      );
    } catch (error) {
      toast({
        message: "Failed to request upload URLs",
        error,
        variant: "error",
      });
      for (const item of items) {
        this.removeItem(item.uid);
      }
      this.refreshControls();
      this.syncHiddenField();
      // The Thumbnails may still be generating; once they settle, revoke their
      // object URLs (the items are already gone, so nothing else will). This
      // also swallows their rejections so they don't surface as unhandled.
      void Promise.allSettled(thumbnailPromises).then(() => {
        for (const item of items) {
          this.revokeThumbnail(item);
        }
      });
      return;
    }

    // The file ids are minted before the blobs land; record them now so the
    // hidden field carries them even while the uploads are still in flight.
    for (const [index, item] of items.entries()) {
      item.fileIdOriginal = uploadParamsList[index].file_id_original;
      item.fileIdThumbnail = uploadParamsList[index].file_id_thumbnail;
    }
    this.syncHiddenField();

    await Promise.allSettled(
      items.map(async (item, index) => {
        await this.uploadItem(
          item,
          files[index],
          thumbnailPromises[index],
          uploadParamsList[index],
        );
      }),
    );
  }

  private async uploadItem(
    item: EditorItem,
    file: File,
    thumbnailPromise: Promise<ThumbnailFromAnyResult>,
    uploadParams: MediaUploadUrlResultItem,
  ): Promise<void> {
    try {
      const thumbnailResult = await thumbnailPromise;
      await Promise.all([
        uploadOne(
          file,
          uploadParams.upload_method,
          uploadParams.upload_url_original,
          uploadParams.upload_headers_original,
        ),
        uploadOne(
          thumbnailResult.thumbnail,
          uploadParams.upload_method,
          uploadParams.upload_url_thumbnail,
          uploadParams.upload_headers_thumbnail,
        ),
      ]);
      item.status = "ready";
    } catch (error) {
      item.status = "error";
      toast({
        message: `Failed to upload ${file.name}`,
        error,
        variant: "error",
      });
    }
    this.updateStatus(item);
  }

  private async generateThumbnail(
    item: EditorItem,
    file: File,
  ): Promise<ThumbnailFromAnyResult> {
    const result =
      item.mediaType === "video"
        ? await thumbnailFromVideo(file)
        : await thumbnailFromImage(file);
    item.widthOriginal = result.widthOriginal;
    item.heightOriginal = result.heightOriginal;
    item.widthThumbnail = result.widthThumbnail;
    item.heightThumbnail = result.heightThumbnail;
    item.thumbnailUrl = URL.createObjectURL(result.thumbnail);
    this.updateThumbnail(item);
    this.syncHiddenField();
    return result;
  }

  // --- In-memory mutations -------------------------------------------------

  private onReorder(item: EditorItem, direction: "up" | "down"): void {
    const index = this.items.indexOf(item);
    const targetIndex = direction === "up" ? index - 1 : index + 1;
    if (targetIndex < 0 || targetIndex >= this.items.length) {
      return;
    }
    this.swapItems(index, targetIndex);
    this.dirty = true;
    this.syncHiddenField();
  }

  private onDelete(item: EditorItem): void {
    if (!window.confirm("Are you sure you wish to delete this?")) {
      return;
    }
    this.removeItem(item.uid);
    this.dirty = true;
    this.refreshControls();
    this.syncHiddenField();
  }

  // --- Rendering -----------------------------------------------------------

  private createPendingItem(file: File): EditorItem {
    return {
      uid: this.uidCounter++,
      mediaType: file.type.startsWith("video/") ? "video" : "image",
      caption: "",
      fileIdOriginal: -1,
      widthOriginal: 0,
      heightOriginal: 0,
      fileIdThumbnail: -1,
      widthThumbnail: 0,
      heightThumbnail: 0,
      thumbnailUrl: "",
      status: "pending",
    };
  }

  private itemFromWire(wire: MediaEditorItem): EditorItem {
    return {
      uid: this.uidCounter++,
      mediaType: wire.media_type,
      caption: wire.caption,
      fileIdOriginal: wire.file_id_original,
      widthOriginal: wire.width_original,
      heightOriginal: wire.height_original,
      fileIdThumbnail: wire.file_id_thumbnail,
      widthThumbnail: wire.width_thumbnail,
      heightThumbnail: wire.height_thumbnail,
      thumbnailUrl: wire.url_thumbnail,
      status: "ready",
    };
  }

  private toWire(item: EditorItem): MediaEditorItem {
    return {
      media_type: item.mediaType,
      caption: item.caption,
      file_id_original: item.fileIdOriginal,
      width_original: item.widthOriginal,
      height_original: item.heightOriginal,
      file_id_thumbnail: item.fileIdThumbnail,
      width_thumbnail: item.widthThumbnail,
      height_thumbnail: item.heightThumbnail,
      url_thumbnail: "",
    };
  }

  private renderItem(item: EditorItem): void {
    const fragment = this.getTarget("itemTemplate").content.cloneNode(
      true,
    ) as DocumentFragment;
    const root = fragment.querySelector<HTMLElement>("[data-editor-item]");
    if (!root) throw new Error("Item template missing [data-editor-item]");

    const els: ItemElements = {
      root,
      thumb: queryRef(root, "thumb") as HTMLImageElement,
      videoIndicator: queryRef(root, "videoIndicator"),
      spinner: queryRef(root, "spinner"),
      caption: queryRef(root, "caption") as HTMLInputElement,
      up: queryRef(root, "up") as HTMLButtonElement,
      down: queryRef(root, "down") as HTMLButtonElement,
      del: queryRef(root, "delete") as HTMLButtonElement,
    };

    els.caption.value = item.caption;
    // Caption edits are read from the DOM at sync time (see syncHiddenField);
    // the form's `input` listener marks the form dirty.
    els.up.addEventListener("click", () => {
      this.onReorder(item, "up");
    });
    els.down.addEventListener("click", () => {
      this.onReorder(item, "down");
    });
    els.del.addEventListener("click", () => {
      this.onDelete(item);
    });

    this.elements.set(item.uid, els);
    this.getTarget("list").append(root);

    this.updateThumbnail(item);
    this.updateStatus(item);
  }

  private updateThumbnail(item: EditorItem): void {
    const els = this.elements.get(item.uid);
    if (!els) return;
    if (item.thumbnailUrl) {
      els.thumb.src = item.thumbnailUrl;
    }
    if (item.widthThumbnail) {
      els.thumb.width = item.widthThumbnail;
      els.thumb.height = item.heightThumbnail;
    }
    els.videoIndicator.classList.toggle("hidden", item.mediaType !== "video");
  }

  private updateStatus(item: EditorItem): void {
    const els = this.elements.get(item.uid);
    if (!els) return;
    const isError = item.status === "error";
    const isPending = item.status === "pending";
    els.spinner.classList.toggle("hidden", !isPending);
    els.root.classList.toggle("opacity-50", isPending);
    // Flag failed uploads so the user can spot and remove them.
    els.root.classList.toggle("ring-2", isError);
    els.root.classList.toggle("ring-error", isError);
  }

  private refreshControls(): void {
    for (const [index, item] of this.items.entries()) {
      const els = this.elements.get(item.uid);
      if (!els) continue;
      els.up.disabled = index === 0;
      els.down.disabled = index === this.items.length - 1;
    }
  }

  private swapItems(a: number, b: number): void {
    [this.items[a], this.items[b]] = [this.items[b], this.items[a]];
    const list = this.getTarget("list");
    const lo = Math.min(a, b);
    const loEls = this.elements.get(this.items[lo].uid);
    const hiEls = this.elements.get(this.items[lo + 1].uid);
    // Move the now-lower node before the now-higher one.
    if (loEls && hiEls) {
      list.insertBefore(loEls.root, hiEls.root);
    }
    this.refreshControls();
  }

  private removeItem(uid: number): void {
    const index = this.items.findIndex((item) => item.uid === uid);
    if (index === -1) return;
    const [item] = this.items.splice(index, 1);
    this.revokeThumbnail(item);
    const els = this.elements.get(uid);
    els?.root.remove();
    this.elements.delete(uid);
  }

  private revokeThumbnail(item: EditorItem): void {
    if (item.thumbnailUrl.startsWith("blob:")) {
      URL.revokeObjectURL(item.thumbnailUrl);
    }
  }

  private syncHiddenField(): void {
    // Pull each caption straight from its live input so an un-blurred edit is
    // captured even if no `change` event has fired yet.
    for (const item of this.items) {
      const els = this.elements.get(item.uid);
      if (els) item.caption = els.caption.value;
    }
    this.getTarget("hiddenField").value = JSON.stringify(
      this.items.map((item) => this.toWire(item)),
    );
  }

  // --- Submit gate & unsaved-changes guard --------------------------------

  private markDirty = (): void => {
    this.dirty = true;
  };

  // Holds the form submit until in-flight uploads land (or fail).
  private onConfirm = (event: Event): void => {
    const evt = event as HtmxConfirmEvent;
    // Only gate this form's own submit, not e.g. the Publish button.
    if (evt.detail.elt !== this.form) return;
    // Capture any un-blurred caption edits before the form is submitted.
    this.syncHiddenField();
    if (!this.hasPendingUploads() && !this.hasFailedUploads()) {
      // Nothing in flight -- let the submit proceed normally.
      return;
    }
    evt.preventDefault();

    this.setSubmitting(true);
    void this.settleBatches().then((settled) => {
      this.setSubmitting(false);
      if (!settled) {
        // An upload is taking too long (or has stalled). Don't block the page;
        // the uploads keep running, so the user can retry once they land.
        toast({
          message: "Uploads are still finishing. Try again in a moment.",
          variant: "error",
        });
        return;
      }
      if (this.hasFailedUploads()) {
        toast({
          message: "Some uploads failed. Please remove them and try again.",
          variant: "error",
        });
        return;
      }
      this.syncHiddenField();
      evt.detail.issueRequest(true);
    });
  };

  private onAfterRequest = (event: Event): void => {
    const evt = event as HtmxAfterRequestEvent;
    // Only the form's own submit clears the dirty flag (not the Publish button).
    if (evt.detail.elt === this.form && evt.detail.successful) {
      this.dirty = false;
    }
  };

  private onBeforeUnload = (event: BeforeUnloadEvent): void => {
    if (this.dirty || this.hasPendingUploads()) {
      event.preventDefault();
      // eslint-disable-next-line @typescript-eslint/no-deprecated -- Still required to trigger the prompt in some browsers.
      event.returnValue = "";
    }
  };

  // Awaits all added batches (re-checking for batches queued while awaiting),
  // bounded by a cap so a stalled upload can't freeze the submit forever.
  // Resolves true if everything settled, false if the cap was hit first.
  private async settleBatches(): Promise<boolean> {
    let timer: ReturnType<typeof setTimeout> | undefined;
    const cap = new Promise<false>((resolve) => {
      timer = setTimeout(() => {
        resolve(false);
      }, UPLOAD_SETTLE_TIMEOUT_MS);
    });
    const all = (async () => {
      // `batches` shrinks as batches settle (and may grow if more are added
      // mid-wait), so loop until it is empty.
      while (this.batches.length > 0) {
        // allSettled materializes the iterable synchronously, so this snapshots
        // the current batches even though more may be added later.
        await Promise.allSettled(this.batches);
      }
      return true as const;
    })();

    const settled = await Promise.race([all, cap]);
    clearTimeout(timer);
    return settled;
  }

  private hasPendingUploads(): boolean {
    return this.items.some((item) => item.status === "pending");
  }

  private hasFailedUploads(): boolean {
    return this.items.some((item) => item.status === "error");
  }

  private setSubmitting(on: boolean): void {
    const button = this.form?.querySelector<HTMLButtonElement>(
      'button[type="submit"]',
    );
    if (!button) return;
    button.disabled = on;
    button.querySelector(".loading")?.classList.toggle("hidden", !on);
  }
}

function queryRef(root: HTMLElement, ref: string): HTMLElement {
  const el = root.querySelector<HTMLElement>(`[data-ref="${ref}"]`);
  if (!el) throw new Error(`Item template missing [data-ref="${ref}"]`);
  return el;
}

async function fetchUploadUrls(
  files: File[],
  hrefUploadUrl: string,
): Promise<MediaUploadUrlResultItem[]> {
  const body: MediaUploadUrlBody = {
    thumbnail_extension: THUMBNAIL_EXT,
    filenames: files.map((file) => file.name),
  };
  const resp = await fetch(hrefUploadUrl, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!resp.ok) {
    throw new Error(`Request failed with status ${resp.status}`);
  }
  return z.array(mediaUploadUrlResultItemSchema).parse(await resp.json());
}

async function uploadOne(
  file: File | Blob,
  method: string,
  url: string,
  headers: Record<string, string>,
): Promise<void> {
  const resp = await fetch(url, {
    method,
    body: file,
    headers,
  });
  if (!resp.ok) {
    throw new Error(`Request failed with status ${resp.status}`);
  }
}
