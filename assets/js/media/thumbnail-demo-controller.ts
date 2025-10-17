import { TypedController } from "../utils/stimulus-typed";
import { thumbnailFromImage, thumbnailFromVideo } from "./thumbnail";

export class ThumbnailDemoController extends TypedController(
  "media--thumbnail-demo",
  "div",
  {
    targets: {
      fileInput: "input",
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
      this.clearLogs();
      try {
        await this.handleFileInput($fileInput);
      } catch (error) {
        const errorMsg = error instanceof Error ? error.message : String(error);
        this.log("ERROR: " + errorMsg);
      }
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

  async handleFileInput($fileInput: HTMLInputElement): Promise<void> {
    const file = $fileInput.files?.item(0);
    if (!file) {
      return;
    }

    this.log(`media type: ${file.type}`);
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
