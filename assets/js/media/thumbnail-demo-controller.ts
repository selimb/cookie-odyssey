import { TypedController } from "../utils/stimulus-typed";
import { probeMediaDate } from "./media-date-probe";
import { thumbnailFromImage, thumbnailFromVideo } from "./thumbnail";

export class ThumbnailDemoController extends TypedController(
  "media--thumbnail-demo",
  "div",
  {
    targets: {
      fileInput: "input",
      dropzone: "div",
      logs: "ul",
      thumbnail: "img",
    },
  },
) {
  private start = Date.now();

  connect(): void {
    const $fileInput = this.getTarget("fileInput");

    // eslint-disable-next-line @typescript-eslint/no-misused-promises -- Hush.
    $fileInput.addEventListener("change", async () => {
      await this.handleFiles([...($fileInput.files ?? [])], "file input");
    });

    const $dropzone = this.getTarget("dropzone");
    // Without preventDefault on dragover the browser refuses the drop.
    $dropzone.addEventListener("dragover", (event) => {
      event.preventDefault();
    });
    // eslint-disable-next-line @typescript-eslint/no-misused-promises -- Hush.
    $dropzone.addEventListener("drop", async (event) => {
      event.preventDefault();
      const files = [...(event.dataTransfer?.files ?? [])];
      await this.handleFiles(files, "drop");
    });
  }

  clearLogs(): void {
    this.getTarget("logs").innerHTML = "";
    this.start = Date.now();
  }

  log = (message: string): void => {
    const elapsed = Date.now() - this.start;

    const $logItem = document.createElement("li");
    $logItem.textContent = `[${elapsed}] ${message}`;

    this.getTarget("logs").append($logItem);
  };

  async handleFiles(files: File[], via: string): Promise<void> {
    this.clearLogs();
    if (files.length === 0) {
      return;
    }
    this.log(`received ${files.length} file(s) via ${via}`);

    for (const file of files) {
      this.log(`--- ${file.name} ---`);
      this.log(`type: ${file.type || "(empty)"}`);
      this.log(`size: ${file.size}`);
      // The whole point of the diagnostic: compare lastModified against the
      // capture date the format actually carries.
      this.log(
        `lastModified (local): ${new Date(file.lastModified).toString()}`,
      );
      this.log(
        `lastModified (ISO): ${new Date(file.lastModified).toISOString()}`,
      );
      try {
        const probe = await probeMediaDate(file);
        this.log(`format: ${probe.format}`);
        this.log(
          `exif DateTimeOriginal: ${probe.exifDateTimeOriginal ?? "(none)"}`,
        );
        this.log(`video creationTime: ${probe.videoCreationTime ?? "(none)"}`);
      } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error);
        this.log(`probe ERROR: ${errorMsg}`);
      }
    }

    // Keep the original demo behaviour: show a thumbnail for the first file.
    try {
      await this.showThumbnail(files[0]);
    } catch (error) {
      const errorMsg = error instanceof Error ? error.message : String(error);
      this.log(`thumbnail ERROR: ${errorMsg}`);
    }
  }

  async showThumbnail(file: File): Promise<void> {
    const mediaType = file.type.startsWith("video/") ? "video" : "image";

    const thumbnail =
      mediaType === "video"
        ? await thumbnailFromVideo(file, this.log)
        : await thumbnailFromImage(file);
    this.log(
      `generated thumbnail: ${thumbnail.widthOriginal}x${thumbnail.heightOriginal} to ${thumbnail.widthThumbnail}x${thumbnail.heightThumbnail}`,
    );

    const thumbnailUrl = URL.createObjectURL(thumbnail.thumbnail);
    this.getTarget("thumbnail").src = thumbnailUrl;
  }
}
