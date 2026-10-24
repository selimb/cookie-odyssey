import { thumbnailFromImage } from "./media/thumbnail";
import { fetchUploadUrls } from "./media/upload-url";
import { toast } from "./toast";
import { TypedController } from "./utils/stimulus-typed";

/**
 * Uploads a downscaled copy of the chosen image as the Journal's cover.
 * The full-size original is never needed, which keeps the cover light on
 * mobile connections.
 */
export class JournalCoverController extends TypedController(
  "journal-cover",
  "form",
  {
    targets: {
      fileInput: "input",
      fileId: "input",
      preview: "img",
      submit: "button",
      remove: "button",
    },
    values: {
      getUploadUrl: "string",
    },
  },
) {
  connect(): void {
    this.getTarget("remove").addEventListener("click", () => {
      this.removeCover();
    });
    const $fileInput = this.getTarget("fileInput");
    // eslint-disable-next-line @typescript-eslint/no-misused-promises -- Hush.
    $fileInput.addEventListener("change", async () => {
      // Everything is disabled during the upload, so that a second upload
      // cannot race the first, and so that Remove cannot be undone by a
      // finishing upload.
      const $controls = [
        $fileInput,
        this.getTarget("submit"),
        this.getTarget("remove"),
      ];
      for (const $control of $controls) {
        $control.disabled = true;
      }
      try {
        await this.handleFileInput($fileInput);
      } finally {
        for (const $control of $controls) {
          $control.disabled = false;
        }
      }
    });
  }

  removeCover(): void {
    this.getTarget("fileId").value = "";
    this.getTarget("fileInput").value = "";
    this.getTarget("preview").classList.add("hidden");
    this.getTarget("remove").classList.add("hidden");
  }

  async handleFileInput($fileInput: HTMLInputElement): Promise<void> {
    const file = $fileInput.files?.[0];
    if (!file) {
      return;
    }
    const $fileId = this.getTarget("fileId");
    // The id is only written on success, so a failed upload keeps the old cover.
    try {
      const result = await thumbnailFromImage(file);
      const [params] = await fetchUploadUrls(
        [file.name],
        this.getValue("getUploadUrl"),
      );
      const uploadResp = await fetch(params.upload_url_thumbnail, {
        method: params.upload_method,
        body: result.thumbnail,
        headers: params.upload_headers_thumbnail,
      });
      if (!uploadResp.ok) {
        throw new Error(`Upload failed with status ${uploadResp.status}`);
      }
      $fileId.value = String(params.file_id_thumbnail);

      const $preview = this.getTarget("preview");
      $preview.src = URL.createObjectURL(result.thumbnail);
      $preview.classList.remove("hidden");
      this.getTarget("remove").classList.remove("hidden");
    } catch (error) {
      $fileInput.value = "";
      toast({
        message: "Failed to upload cover photo",
        error,
        variant: "error",
      });
    }
  }
}
