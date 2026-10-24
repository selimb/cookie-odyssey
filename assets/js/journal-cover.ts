import { THUMBNAIL_EXT, thumbnailFromImage } from "./media/thumbnail";
import { toast } from "./toast";
import { TypedController } from "./utils/stimulus-typed";

// SYNC MediaUploadUrlBody
type MediaUploadUrlBody = {
  thumbnail_extension: string;
  filenames: string[];
};

// SYNC MediaUploadUrlResultItem
type MediaUploadUrlResultItem = {
  upload_method: string;
  upload_url_original: string;
  upload_url_thumbnail: string;
  upload_headers_original: Record<string, string>;
  upload_headers_thumbnail: Record<string, string>;
  file_id_original: number;
  file_id_thumbnail: number;
};

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
    },
    values: {
      getUploadUrl: "string",
    },
  },
) {
  connect(): void {
    const $fileInput = this.getTarget("fileInput");
    // eslint-disable-next-line @typescript-eslint/no-misused-promises -- Hush.
    $fileInput.addEventListener("change", async () => {
      const $submit = this.getTarget("submit");
      $submit.disabled = true;
      try {
        await this.handleFileInput($fileInput);
      } finally {
        $submit.disabled = false;
      }
    });
  }

  async handleFileInput($fileInput: HTMLInputElement): Promise<void> {
    const file = $fileInput.files?.[0];
    if (!file) {
      return;
    }
    const $fileId = this.getTarget("fileId");
    $fileId.value = "";
    try {
      const result = await thumbnailFromImage(file);
      const body: MediaUploadUrlBody = {
        thumbnail_extension: THUMBNAIL_EXT,
        filenames: [file.name],
      };
      const resp = await fetch(this.getValue("getUploadUrl"), {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(body),
      });
      if (!resp.ok) {
        throw new Error(`Request failed with status ${resp.status}`);
      }
      const [params] = (await resp.json()) as MediaUploadUrlResultItem[];
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
