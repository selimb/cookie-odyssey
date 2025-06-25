import { Controller } from "@hotwired/stimulus";
import {
  THUMBNAIL_EXT,
  thumbnailFromImage,
  thumbnailFromVideo,
  type ThumbnailFromAnyResult,
} from "./thumbnail";
import { defineTargets, defineValues } from "../stimulus-utils";
import { toast } from "../toast";
import htmx from "htmx.org";

const { targets, getTarget } = defineTargets({
  mediaContainer: "div",
  addMediaButton: "button",
  fileInput: "input",
});

const { values, getValue } = defineValues({
  getUploadUrl: "string",
  commitUploadUrl: "string",
  entryId: "number",
});

export class JournalEntryMediaFormController extends Controller {
  static identifier = "media--form";

  static targets = targets;
  getTarget = getTarget;

  static values = values;
  getValue = getValue;

  connect(): void {
    const $mediaButton = this.getTarget("addMediaButton");
    const $mediaButtonSpinner = $mediaButton.querySelector(".loading");

    const $fileInput = this.getTarget("fileInput");

    $mediaButton.addEventListener("click", (event) => {
      event.preventDefault();
      $fileInput.click();
    });

    $fileInput.addEventListener("change", async () => {
      // TODO: Generic?
      $mediaButton.disabled = true;
      $mediaButtonSpinner?.classList.remove("hidden");
      try {
        await this.handleFileInput($fileInput);
      } finally {
        $mediaButton.disabled = false;
        $mediaButtonSpinner?.classList.add("hidden");
      }
    });
  }

  async handleFileInput($fileInput: HTMLInputElement) {
    if (!$fileInput.files || $fileInput.files.length === 0) {
      return;
    }
    const files = [...$fileInput.files];
    const dataGetUploadUrl = this.getValue("getUploadUrl");
    const dataCommitUploadUrl = this.getValue("commitUploadUrl");
    const dataEntryId = this.getValue("entryId");
    const $mediaContainer = this.getTarget("mediaContainer");

    const mediaTypes: MediaType[] = files.map((file) =>
      file.type.startsWith("video/") ? "video" : "image",
    );

    // Kick off thumbnail generation in the background while we wait for upload URLs.
    const thumbnailPromises = files.map(async (file, index) => {
      const mediaType = mediaTypes[index];
      switch (mediaType) {
        case "video":
          return await thumbnailFromVideo(file);
        case "image":
          return await thumbnailFromImage(file);
        default:
          // Should never happen, thanks to the `accept` attribute on the input.
          throw new Error(
            `Unsupported media type ${mediaType} for file ${file.name}`,
          );
      }
    });

    try {
      var uploadParamsList = await fetchUploadUrls(files, dataGetUploadUrl);
    } catch (error) {
      toast({
        message: "Failed to request upload URLs",
        error,
        variant: "error",
      });
      return;
    }

    // XXX
    console.info("uploadParamsList", uploadParamsList);

    const commitItems: JournalEntryMediaCommitItem[] = new Array(
      uploadParamsList.length,
    );

    let hasError = false;
    await Promise.all(
      uploadParamsList.map(async (uploadParams, index) => {
        const file = files[index];
        const mediaType = mediaTypes[index];

        try {
          var thumbnailResult = await uploadOriginalAndThumbnail(
            file,
            thumbnailPromises[index],
            uploadParams,
          );
        } catch (error) {
          toast({
            message: `Failed to upload ${file.name}`,
            error,
            variant: "error",
          });
          hasError = true;
          return;
        }

        commitItems[index] = {
          media_type: mediaType,
          file_id_original: uploadParams.file_id_original,
          width_original: thumbnailResult.widthOriginal,
          height_original: thumbnailResult.heightOriginal,
          file_id_thumbnail: uploadParams.file_id_thumbnail,
          width_thumbnail: thumbnailResult.widthThumbnail,
          height_thumbnail: thumbnailResult.heightThumbnail,
        };
      }),
    );
    if (hasError) {
      return;
    }

    const commitBody: JournalEntryMediaCommitBody = {
      entry_id: Number(dataEntryId),
      items: commitItems,
    };
    try {
      await doCommit(dataCommitUploadUrl, commitBody, $mediaContainer);
    } catch (error) {
      toast({
        message: "Failed to commit files",
        error,
        variant: "error",
      });
      return;
    }
  }
}

// SYNC
type MediaType = "image" | "video";

// SYNC
interface MediaUploadUrlBody {
  thumbnail_extension: string;
  filenames: string[];
}

// SYNC
interface MediaUploadUrlResultItem {
  upload_method: string;
  upload_url_original: string;
  upload_url_thumbnail: string;
  upload_headers_original: Record<string, string>;
  upload_headers_thumbnail: Record<string, string>;
  file_id_original: number;
  file_id_thumbnail: number;
}

// SYNC
interface JournalEntryMediaCommitItem {
  media_type: MediaType;
  file_id_original: number;
  width_original: number;
  height_original: number;
  file_id_thumbnail: number;
  width_thumbnail: number;
  height_thumbnail: number;
}

// SYNC
interface JournalEntryMediaCommitBody {
  entry_id: number;
  items: JournalEntryMediaCommitItem[];
}

async function fetchUploadUrls(
  files: File[],
  getUploadUrl: string,
): Promise<MediaUploadUrlResultItem[]> {
  const body: MediaUploadUrlBody = {
    thumbnail_extension: THUMBNAIL_EXT,
    filenames: files.map((file) => file.name),
  };
  const resp = await fetch(getUploadUrl, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
    },
    body: JSON.stringify(body),
  });
  if (!resp.ok) {
    throw new Error(`Request failed with status ${resp.status}`);
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

async function uploadOriginalAndThumbnail(
  file: File,
  thumbnailPromise: Promise<ThumbnailFromAnyResult>,
  uploadUrlParams: MediaUploadUrlResultItem,
): Promise<ThumbnailFromAnyResult> {
  // This is a bit complicated, but the goal is to kick off the upload of the original file
  // as fast as possible, i.e. not wait for thumbnail generation.
  const promises: Promise<void>[] = [];

  // Original
  promises.push(
    uploadOne(
      file,
      uploadUrlParams.upload_method,
      uploadUrlParams.upload_url_original,
      uploadUrlParams.upload_headers_original,
    ),
  );

  // Thumbnail
  const thumbnailResult = await thumbnailPromise;
  promises.push(
    uploadOne(
      thumbnailResult.thumbnail,
      uploadUrlParams.upload_method,
      uploadUrlParams.upload_url_thumbnail,
      uploadUrlParams.upload_headers_thumbnail,
    ),
  );

  await Promise.all(promises);
  return thumbnailResult;
}

async function doCommit(
  commitUrl: string,
  body: JournalEntryMediaCommitBody,
  hxTarget: Element,
) {
  htmx.ajax("post", commitUrl, {
    target: hxTarget,
    swap: "outerHTML",
    values: { json: body },
  });
}
