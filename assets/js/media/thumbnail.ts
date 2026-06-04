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
    log?.("Created video element");
    cleanup.push(() => {
      video.remove();
      log?.("Removed video element");
    });
    const objectUrl = URL.createObjectURL(file);
    log?.("Created object URL");
    cleanup.push(() => {
      URL.revokeObjectURL(objectUrl);
      log?.("Revoked object URL");
    });

    video.src = objectUrl;
    video.muted = true;
    // Play inline (not full-screen).
    video.playsInline = true;

    //
    // On most browsers, we can simply:
    // ```
    // await video.play();
    // video.pause();
    // video.currentTime = 0;
    // ```
    // But of course, MacOS Safari likes to be difficult, and forces us to do the event listener dance instead:
    // 1. Load the video, and wait for `loadeddata`.
    // 2. Set `currentTime` to the beginning, and wait for `seeked`.
    // Note that none of this starts playing the video, i.e. `video.paused` is still `true`.
    //
    log?.("Loading video");
    video.load();
    log?.("Waiting for loadeddata");
    await promiseTimeout(
      new Promise<void>((resolve, reject) => {
        video.addEventListener(
          "loadeddata",
          () => {
            log?.("loadeddata fired");
            resolve();
          },
          { once: true },
        );
        video.addEventListener(
          "error",
          (evt) => {
            const msg = `Failed to load video: ${evt.message}`;
            log?.(msg);
            reject(new Error(msg));
          },
          { once: true },
        );
      }),
      5000,
      "video.loadeddata",
    );

    log?.("Waiting for video to seek to 0s");
    await promiseTimeout(
      new Promise<void>((resolve) => {
        video.addEventListener(
          "seeked",
          () => {
            log?.("seeked fired");
            resolve();
          },
          { once: true },
        );
        video.currentTime = 0;
      }),
      5000,
      "video.seeked",
    );

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
