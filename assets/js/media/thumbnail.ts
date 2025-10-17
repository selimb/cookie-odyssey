import { promiseTimeout } from "../utils/promise-timeout";

export const THUMBNAIL_WIDTH = 640;
export const THUMBNAIL_QUALITY = 0.8;
export const THUMBNAIL_EXT = ".jpeg";
export const THUMBNAIL_MIME_TYPE = "image/jpeg";

/**
 * Generates a thumbnail from the given media element.
 *
 * @param width Width of the original video/image.
 * @param height Height of the original video/image.
 * @returns {Promise<GenerateThumbnailResult>}
 */
async function generateThumbnail(
  elem: HTMLVideoElement | HTMLImageElement,
  width: number,
  height: number,
): Promise<{ blob: Blob; width: number; height: number }> {
  const canvas = document.createElement("canvas");

  try {
    const aspectRatio = width / height;
    const thumbnailWidth = THUMBNAIL_WIDTH;
    const thumbnailHeight = Math.round(thumbnailWidth / aspectRatio);

    canvas.width = thumbnailWidth;
    canvas.height = thumbnailHeight;
    const ctx = canvas.getContext("2d");
    if (!ctx) {
      throw new Error("Could not get canvas context");
    }
    ctx.drawImage(elem, 0, 0, thumbnailWidth, thumbnailHeight);

    const blob = await promiseTimeout(
      new Promise<Blob | null>((resolve) => {
        canvas.toBlob(
          (blob) => {
            resolve(blob);
          },
          THUMBNAIL_MIME_TYPE,
          THUMBNAIL_QUALITY,
        );
      }),
      5000,
      "canvas.toBlob",
    );
    if (!blob) {
      throw new Error("Could not create thumbnail blob");
    }

    return { blob, width: thumbnailWidth, height: thumbnailHeight };
  } finally {
    canvas.remove();
  }
}

export type ThumbnailFromAnyResult = {
  thumbnail: Blob;
  widthOriginal: number;
  heightOriginal: number;
  widthThumbnail: number;
  heightThumbnail: number;
};

/**
 * Extracts a thumbnail from a video's first frame.
 */
export async function thumbnailFromVideo(
  file: File,
  log?: (msg: string) => void,
): Promise<ThumbnailFromAnyResult> {
  const cleanup: Array<() => void> = [];
  try {
    const video = document.createElement("video");
    cleanup.push(() => {
      video.remove();
    });
    const objectUrl = URL.createObjectURL(file);
    cleanup.push(() => {
      URL.revokeObjectURL(objectUrl);
    });

    video.src = objectUrl;
    video.muted = true;
    video.playsInline = true;
    // NOTE: I tried carefully orchestrating loadedmetadata/loadeddata/seek listeners,
    // but the most reliable way seems to be just calling `.play()`.
    // We already have the video in-memory, so we don't really need to care about over-loading.
    await promiseTimeout(video.play(), 5000, "video.play()");
    // Pause and seek back to beginning.
    video.pause();
    video.currentTime = 0;

    const widthOriginal = video.videoWidth;
    const heightOriginal = video.videoHeight;
    if (widthOriginal === 0 || heightOriginal === 0) {
      throw new Error("Could not get video dimensions");
    }

    log?.(`video dimensions: ${widthOriginal}x${heightOriginal}`);

    const result = await generateThumbnail(
      video,
      widthOriginal,
      heightOriginal,
    );

    return {
      heightOriginal,
      widthOriginal,
      thumbnail: result.blob,
      widthThumbnail: result.width,
      heightThumbnail: result.height,
    };
  } finally {
    for (const fn of cleanup) {
      fn();
    }
  }
}

/**
 * Extracts a thumbnail from an image.
 */
export async function thumbnailFromImage(
  file: File,
): Promise<ThumbnailFromAnyResult> {
  const cleanup: Array<() => void> = [];
  try {
    const img = document.createElement("img");
    cleanup.push(() => {
      img.remove();
    });
    const objectUrl = URL.createObjectURL(file);
    cleanup.push(() => {
      URL.revokeObjectURL(objectUrl);
    });

    img.src = objectUrl;
    // Wait until the image is loaded.
    await img.decode();

    const widthOriginal = img.naturalWidth;
    const heightOriginal = img.naturalHeight;
    const result = await generateThumbnail(img, widthOriginal, heightOriginal);

    return {
      heightOriginal,
      widthOriginal,
      thumbnail: result.blob,
      widthThumbnail: result.width,
      heightThumbnail: result.height,
    };
  } finally {
    for (const fn of cleanup) {
      fn();
    }
  }
}
