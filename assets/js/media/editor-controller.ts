import { z } from "zod";

import { toast } from "../toast";
import { TypedController } from "../utils/stimulus-typed";
import {
  THUMBNAIL_EXT,
  type ThumbnailFromAnyResult,
  thumbnailFromImage,
  thumbnailFromVideo,
} from "./thumbnail";

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
  id: z.number().nullable(),
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

// SYNC MediaCommitBody
type MediaCommitBody = {
  entry_id: number;
  items: MediaEditorItem[];
};

// SYNC MediaCommitResult
const mediaCommitResultSchema = z.object({
  ids: z.array(z.number()),
});

// SYNC MediaCaptionBody
type MediaCaptionBody = {
  media_id: number;
  caption: string;
};

// SYNC Direction
type Direction = "up" | "down";

// SYNC MediaReorderBody
type MediaReorderBody = {
  media_id: number;
  entry_id: number;
  order: number;
  direction: Direction;
};

// SYNC MediaDeleteBody
type MediaDeleteBody = {
  media_id: number;
};

// The editor's in-memory model of a single Media item. Richer than the wire
// `MediaEditorItem`: it tracks the client-side `uid`, upload `status`, and a
// renderable `thumbnailUrl` (a local object URL for freshly-added files, a
// signed URL for items seeded from the server).
type EditorItem = {
  uid: number;
  // The persisted `JournalEntryMedia` id, or null until the item is created.
  serverId: number | null;
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
// that was held back with `preventDefault()`.
type HtmxConfirmEvent = CustomEvent<{
  issueRequest: (skipConfirmation?: boolean) => void;
}>;

/**
 * A single client-owned Media editor, used on both the new-entry and edit
 * pages. It owns an in-memory ordered list of Media and renders the Thumbnails
 * itself.
 *
 * The presence of an `entryId` selects the persistence mode:
 * - Edit mode (entryId present): every mutation persists immediately.
 * - New-entry mode (no entryId): mutations only touch the in-memory list, which
 *   is serialized into a hidden form field so the page's normal HTMX form
 *   submission carries it. A `htmx:confirm` gate makes Create wait for in-flight
 *   uploads to land.
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
    },
    values: {
      hrefUploadUrl: "string",
      hrefCommit: "string",
      hrefCaption: "string",
      hrefReorder: "string",
      hrefDelete: "string",
      initialItems: "string",
    },
  },
) {
  private items: EditorItem[] = [];
  private elements = new Map<number, ItemElements>();
  private uidCounter = 0;
  // Tracks every in-flight upload so the Create gate can await them. Each
  // promise resolves (never rejects) once its upload settles.
  private uploadPromises: Array<Promise<void>> = [];

  connect(): void {
    const initialItems: MediaEditorItem[] = z
      .array(mediaEditorItemSchema)
      .parse(JSON.parse(this.getValue("initialItems")));
    for (const wire of initialItems) {
      const item = this.itemFromWire(wire);
      this.items.push(item);
      this.renderItem(item);
    }
    this.refreshControls();
    this.syncHiddenField();

    const $addButton = this.getTarget("addButton");
    const $fileInput = this.getTarget("fileInput");
    const $addSpinner = $addButton.querySelector(".loading");

    $addButton.addEventListener("click", (event) => {
      event.preventDefault();
      $fileInput.click();
    });

    // eslint-disable-next-line @typescript-eslint/no-misused-promises -- Async handler.
    $fileInput.addEventListener("change", async () => {
      if (!$fileInput.files || $fileInput.files.length === 0) {
        return;
      }
      const files = [...$fileInput.files];
      // Reset so re-selecting the same file fires `change` again.
      $fileInput.value = "";

      $addButton.disabled = true;
      $addSpinner?.classList.remove("hidden");
      try {
        await this.handleFiles(files);
      } finally {
        $addButton.disabled = false;
        $addSpinner?.classList.add("hidden");
      }
    });

    if (!this.isEditMode) {
      this.element.addEventListener("htmx:confirm", this.onConfirm);
    }
  }

  disconnect(): void {
    if (!this.isEditMode) {
      this.element.removeEventListener("htmx:confirm", this.onConfirm);
    }
    for (const item of this.items) {
      this.revokeThumbnail(item);
    }
  }

  private get entryId(): number | null {
    const raw = this.element.getAttribute("data-media--editor-entry-id-value");
    return raw ? Number(raw) : null;
  }

  private get isEditMode(): boolean {
    return this.entryId !== null;
  }

  private get hiddenField(): HTMLInputElement | null {
    return this.element.querySelector(
      `[data-media--editor-target="hiddenField"]`,
    );
  }

  private get createButton(): HTMLButtonElement | null {
    return this.element.querySelector(
      `[data-media--editor-target="createButton"]`,
    );
  }

  private async handleFiles(files: File[]): Promise<void> {
    const items = files.map((file) => this.createPendingItem(file));
    for (const item of items) {
      this.items.push(item);
      this.renderItem(item);
    }
    this.refreshControls();

    // Kick off Thumbnail generation right away so a preview appears
    // immediately, in parallel with requesting upload URLs.
    const thumbnailPromises = files.map(async (file, index) =>
      await this.generateThumbnail(items[index], file),
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
      return;
    }

    // The file ids are minted before the blobs land; record them now so the
    // hidden field carries them even while the uploads are still in flight.
    for (const [index, item] of items.entries()) {
      item.fileIdOriginal = uploadParamsList[index].file_id_original;
      item.fileIdThumbnail = uploadParamsList[index].file_id_thumbnail;
    }
    this.syncHiddenField();

    // Upload each file independently in the background.
    const uploadPromises = items.map(async (item, index) =>
      { await this.uploadItem(
        item,
        files[index],
        thumbnailPromises[index],
        uploadParamsList[index],
      ); },
    );
    this.uploadPromises.push(...uploadPromises);

    if (this.isEditMode) {
      await this.commitAfterUpload(items, uploadPromises);
    }
    // In new-entry mode, `uploadItem` flips each item's status and re-syncs the
    // hidden field on its own; the Create gate awaits `uploadPromises`.
  }

  private createPendingItem(file: File): EditorItem {
    return {
      uid: this.uidCounter++,
      serverId: null,
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
      serverId: wire.id,
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
      id: item.serverId,
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
    this.refreshControls();
    this.syncHiddenField();
  }

  // Edit mode only: after a batch of uploads settles, persist the successful
  // ones and adopt their server ids.
  private async commitAfterUpload(
    items: EditorItem[],
    uploadPromises: Array<Promise<void>>,
  ): Promise<void> {
    await Promise.allSettled(uploadPromises);

    for (const item of items) {
      if (item.status === "error") {
        this.removeItem(item.uid);
      }
    }
    const ready = items.filter((item) => this.items.includes(item));
    if (ready.length === 0) {
      this.refreshControls();
      return;
    }

    let ids: number[];
    try {
      ids = await this.commitItems(ready);
    } catch (error) {
      toast({ message: "Failed to save media", error, variant: "error" });
      for (const item of ready) {
        this.removeItem(item.uid);
      }
      this.refreshControls();
      this.syncHiddenField();
      return;
    }

    for (const [index, item] of ready.entries()) {
      item.serverId = ids[index];
      this.updateStatus(item);
    }
    this.refreshControls();
  }

  private async commitItems(items: EditorItem[]): Promise<number[]> {
    const entryId = this.entryId;
    if (entryId === null) {
      throw new Error("Cannot commit without an entry id");
    }
    const body: MediaCommitBody = {
      entry_id: entryId,
      items: items.map((item) => this.toWire(item)),
    };
    const json = await postJson(this.getValue("hrefCommit"), body);
    return mediaCommitResultSchema.parse(json).ids;
  }

  // --- Mutations -----------------------------------------------------------

  private onCaptionChange(item: EditorItem): void {
    const els = this.elements.get(item.uid);
    if (!els) return;
    item.caption = els.caption.value;
    this.syncHiddenField();

    if (this.isEditMode && item.serverId !== null) {
      const body: MediaCaptionBody = {
        media_id: item.serverId,
        caption: item.caption,
      };
      void postJson(this.getValue("hrefCaption"), body)
        .then(() => {
          toast({ message: "Caption saved", variant: "success" });
        })
        .catch((error: unknown) => {
          toast({ message: "Failed to save caption", error, variant: "error" });
        });
    }
  }

  private async onReorder(
    item: EditorItem,
    direction: Direction,
  ): Promise<void> {
    const index = this.items.indexOf(item);
    const targetIndex = direction === "up" ? index - 1 : index + 1;
    if (targetIndex < 0 || targetIndex >= this.items.length) {
      return;
    }

    // The server swaps by `order`, which mirrors the array index since the
    // client renders in persisted order.
    if (this.isEditMode && item.serverId !== null) {
      const body: MediaReorderBody = {
        media_id: item.serverId,
        entry_id: this.entryId ?? 0,
        order: index,
        direction,
      };
      try {
        await postJson(this.getValue("hrefReorder"), body);
      } catch (error) {
        toast({ message: "Failed to reorder media", error, variant: "error" });
        return;
      }
    }

    this.swapItems(index, targetIndex);
    this.syncHiddenField();
  }

  private async onDelete(item: EditorItem): Promise<void> {
    if (!window.confirm("Are you sure you wish to delete this?")) {
      return;
    }

    if (this.isEditMode && item.serverId !== null) {
      const body: MediaDeleteBody = { media_id: item.serverId };
      try {
        await postJson(this.getValue("hrefDelete"), body);
      } catch (error) {
        toast({ message: "Failed to delete media", error, variant: "error" });
        return;
      }
    }

    this.removeItem(item.uid);
    this.refreshControls();
    this.syncHiddenField();
  }

  // --- Rendering -----------------------------------------------------------

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
    els.caption.addEventListener("change", () => {
      this.onCaptionChange(item);
    });
    els.up.addEventListener("click", () => {
      void this.onReorder(item, "up");
    });
    els.down.addEventListener("click", () => {
      void this.onReorder(item, "down");
    });
    els.del.addEventListener("click", () => {
      void this.onDelete(item);
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
    const showSpinner = !this.isSettled(item) && !isError;
    els.spinner.classList.toggle("hidden", !showSpinner);
    els.root.classList.toggle("opacity-50", showSpinner);
    // Flag failed uploads so the Admin can spot and remove them.
    els.root.classList.toggle("ring-2", isError);
    els.root.classList.toggle("ring-error", isError);
  }

  // An item is "settled" once it is safe to reorder it: in edit mode that means
  // it has been persisted (so the server can swap it); in new-entry mode it just
  // means the upload has landed.
  private isSettled(item: EditorItem): boolean {
    return this.isEditMode ? item.serverId !== null : item.status === "ready";
  }

  private refreshControls(): void {
    for (const [index, item] of this.items.entries()) {
      const els = this.elements.get(item.uid);
      if (!els) continue;
      const prev = this.items[index - 1];
      const next = this.items[index + 1];
      // Removing a mistake and editing captions are always allowed; in edit
      // mode a pre-commit caption is carried by the commit call.
      els.del.disabled = false;
      els.caption.disabled = false;
      // A reorder swaps two items, so both ends must be safe to move. In edit
      // mode that means both are persisted; in new-entry mode it is purely an
      // in-memory swap.
      els.up.disabled =
        !this.canMove(item) || index === 0 || !this.canMove(prev);
      els.down.disabled =
        !this.canMove(item) ||
        index === this.items.length - 1 ||
        !this.canMove(next);
    }
  }

  // Whether an item may participate in a reorder swap.
  private canMove(item: EditorItem | undefined): boolean {
    return item !== undefined && (!this.isEditMode || this.isSettled(item));
  }

  private swapItems(a: number, b: number): void {
    [this.items[a], this.items[b]] = [this.items[b], this.items[a]];
    // Re-attach the DOM nodes in the new order.
    const list = this.getTarget("list");
    for (const item of this.items) {
      const els = this.elements.get(item.uid);
      if (els) list.append(els.root);
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
    const field = this.hiddenField;
    if (!field) return;
    field.value = JSON.stringify(this.items.map((item) => this.toWire(item)));
  }

  // --- Create gate (new-entry mode) ---------------------------------------

  private onConfirm = (event: Event): void => {
    const evt = event as HtmxConfirmEvent;
    const pending = this.items.some((item) => item.status === "pending");
    const failed = this.items.some((item) => item.status === "error");

    if (!pending && !failed) {
      // Nothing in flight -- let the submit proceed normally.
      return;
    }
    evt.preventDefault();

    if (!pending) {
      toast({
        message: "Some uploads failed. Please remove them and try again.",
        variant: "error",
      });
      return;
    }

    this.setCreateSpinner(true);
    void Promise.allSettled(this.uploadPromises).then(() => {
      this.setCreateSpinner(false);
      if (this.items.some((item) => item.status === "error")) {
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

  private setCreateSpinner(on: boolean): void {
    const button = this.createButton;
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
  const json = await postJson(hrefUploadUrl, body);
  return z.array(mediaUploadUrlResultItemSchema).parse(json);
}

async function postJson(url: string, body: unknown): Promise<unknown> {
  const resp = await fetch(url, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify(body),
  });
  if (!resp.ok) {
    throw new Error(`Request failed with status ${resp.status}`);
  }
  if (resp.status === 204) {
    return null;
  }
  return await resp.json();
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
