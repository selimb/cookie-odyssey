import { z } from "zod";

import { THUMBNAIL_EXT } from "./thumbnail";

// SYNC MediaUploadUrlBody
type MediaUploadUrlBody = {
  thumbnail_extension: string;
  filenames: string[];
};

// SYNC MediaUploadUrlResultItem
const MediaUploadUrlResultItemSchema = z.object({
  upload_method: z.string(),
  upload_url_original: z.string(),
  upload_url_thumbnail: z.string(),
  upload_headers_original: z.record(z.string(), z.string()),
  upload_headers_thumbnail: z.record(z.string(), z.string()),
  file_id_original: z.number(),
  file_id_thumbnail: z.number(),
});

export type MediaUploadUrlResultItem = z.infer<
  typeof MediaUploadUrlResultItemSchema
>;

/**
 * Requests one set of upload URLs per filename, in the same order.
 */
export async function fetchUploadUrls(
  filenames: string[],
  getUploadUrl: string,
): Promise<MediaUploadUrlResultItem[]> {
  const body: MediaUploadUrlBody = {
    thumbnail_extension: THUMBNAIL_EXT,
    filenames,
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
  const items = z
    .array(MediaUploadUrlResultItemSchema)
    .parse(await resp.json());
  if (items.length !== filenames.length) {
    throw new Error(
      `Expected ${filenames.length} upload URLs, got ${items.length}`,
    );
  }
  return items;
}
