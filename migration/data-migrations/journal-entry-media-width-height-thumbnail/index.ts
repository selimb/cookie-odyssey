#!/usr/bin/env -S bun run
/**
 * One-time script to setup width, height and thumbnails for journal_entry_media.
 *
 * Usage:
 *
 * ```
 * migration/data-migrations/journal-entry-media-width-height-thumbnail/index.ts
 * ```
 */
import fs, { readFileSync } from "node:fs";
import path from "node:path";

import {
  BlobServiceClient,
  StorageSharedKeyCredential,
} from "@azure/storage-blob";
import { $ } from "bun";
import { Database } from "bun:sqlite";
import sharp from "sharp";
import { z } from "zod";

const MEDIA_DOWNLOAD_DIR = "tmp/media/";
fs.mkdirSync(MEDIA_DOWNLOAD_DIR, { recursive: true });

const zEnv = z.object({
  "APP.DATABASE_FILE": z.string(),

  "APP.STORAGE.CONTAINER_MEDIA": z.string(),
  "APP.STORAGE.AZURE_STORAGE_ACCOUNT": z.string(),
  "APP.STORAGE.AZURE_STORAGE_ACCESS_KEY": z.string(),
});
const env = zEnv.parse(process.env);

const db = new Database(env["APP.DATABASE_FILE"], { strict: true });

const logger = console;

const storage = new BlobServiceClient(
  `https://${env["APP.STORAGE.AZURE_STORAGE_ACCOUNT"]}.blob.core.windows.net`,
  new StorageSharedKeyCredential(
    env["APP.STORAGE.AZURE_STORAGE_ACCOUNT"],
    env["APP.STORAGE.AZURE_STORAGE_ACCESS_KEY"],
  ),
);
const containerClient = storage.getContainerClient(
  env["APP.STORAGE.CONTAINER_MEDIA"],
);

const zJournalEntryMedia = z.object({
  id: z.number(),
  media_type: z.enum(["image", "video"]),
  width: z.number(),
  height: z.number(),
  file_id: z.number(),
  thumbnail_width: z.number(),
  thumbnail_height: z.number(),
  thumbnail_file_id: z.number(),
});
type JournalEntryMedia = z.infer<typeof zJournalEntryMedia>;

const zFile = z.object({
  id: z.number(),
  bucket: z.string(),
  key: z.string(),
});
type FileDb = z.infer<typeof zFile>;

function queryMedia(): JournalEntryMedia[] {
  const raw = db.query("SELECT * FROM journal_entry_media").all();
  const rows = z.array(zJournalEntryMedia).parse(raw);
  return rows;
}

function queryFile(fileId: number): FileDb {
  const raw = db.query("SELECT * FROM file WHERE id = $fileId").all({ fileId });
  const rows = z.array(zFile).parse(raw);
  const row = rows.at(0);
  if (!row) {
    throw new Error(`File with id ${fileId} not found`);
  }
  return row;
}

async function downloadFile(fileId: number): Promise<string> {
  const filePath = path.join(MEDIA_DOWNLOAD_DIR, `${fileId}`);
  if (fs.existsSync(filePath)) {
    return filePath;
  }

  const fileDb = queryFile(fileId);
  const blobClient = containerClient.getBlobClient(fileDb.key);
  await blobClient.downloadToFile(filePath);

  return filePath;
}

async function inferDimensions(
  filePath: string,
): Promise<{ width: number; height: number }> {
  let stdout =
    await $`ffprobe -v error -show_entries stream=width,height -of csv=p=0 ${filePath}`.text();
  stdout = stdout.trim();
  try {
    const [width, height] = stdout
      .split(",")
      .map((s) => z.coerce.number().min(1).finite().parse(s));
    return { width, height };
  } catch (error) {
    throw new Error(
      `Failed to parse dimensions from ffprobe output: ${stdout}`,
      { cause: error },
    );
  }
}

async function extractFirstFrame(
  videoPath: string,
  outputPath: string,
): Promise<void> {
  logger.info(`Extracting first frame from video ${videoPath}`);

  // Need to ensure `outputPath` does not exist, otherwise ffmpeg hangs.
  fs.rmSync(outputPath, { force: true });
  await $`ffmpeg -i ${videoPath} -frames:v 1 ${outputPath}`.quiet();
}

async function generateThumbnail(
  inputImagePath: string,
  outputPath: string,
): Promise<void> {
  if (fs.existsSync(outputPath)) {
    return;
  }

  logger.info(`Generating thumbnail for image ${inputImagePath}`);

  await sharp(inputImagePath)
    .jpeg({ quality: 80 })
    .resize(640)
    .toFile(outputPath);
}

async function setThumbnailForMedia(
  media: JournalEntryMedia,
  thumbnailPath: string,
): Promise<void> {
  const fileDb = queryFile(media.file_id);
  const thumbnailKey = `${fileDb.key.split(".")[0]}-thumbnail.jpeg`;
  logger.info("Uploading", thumbnailKey);
  // For some reason `.uploadFile` does not work for some files :facepalm:
  const thumbnailContent = readFileSync(thumbnailPath);
  await containerClient
    .getBlobClient(thumbnailKey)
    .getBlockBlobClient()
    .upload(thumbnailContent, thumbnailContent.length, {
      blobHTTPHeaders: { blobContentType: "image/jpeg" },
    });
  // .uploadFile(thumbnailPath, {
  //   blobHTTPHeaders: { blobContentType: "image/jpeg" },
  // });

  const thumbnailDimensions = await inferDimensions(thumbnailPath);
  const raw = db
    .query("INSERT INTO file (bucket, key) VALUES ($bucket, $key) RETURNING id")
    .get({ bucket: fileDb.bucket, key: thumbnailKey });
  const { id: thumbnailFileId } = zFile.pick({ id: true }).parse(raw);

  db.query(
    `
      UPDATE journal_entry_media
      SET
        thumbnail_file_id = $thumbnailFileId,
        thumbnail_width = $thumbnailWidth,
        thumbnail_height = $thumbnailHeight
      WHERE id = $id
    `,
  ).run({
    thumbnailFileId,
    thumbnailWidth: thumbnailDimensions.width,
    thumbnailHeight: thumbnailDimensions.height,
    id: media.id,
  });
}

async function main(): Promise<void> {
  const mediaList = queryMedia();
  for (const [index, media] of mediaList.entries()) {
    logger.info(`Processing media ${index + 1}/${mediaList.length}`);

    const shouldInferDimensions = media.width === 0 || media.height === 0;
    const shouldGenerateThumbnail = media.thumbnail_file_id === media.file_id;

    if (shouldInferDimensions) {
      const filePath = await downloadFile(media.file_id);
      const dimensions = await inferDimensions(filePath);
      db.query(
        "UPDATE journal_entry_media SET width = $width, height = $height WHERE id = $id",
      ).run({
        width: dimensions.width,
        height: dimensions.height,
        id: media.id,
      });
    }

    if (shouldGenerateThumbnail) {
      const filePath = await downloadFile(media.file_id);
      const thumbnailPath = filePath + ".thumbnail";
      let imagePath = filePath;
      if (media.media_type === "video") {
        // For videos, we need to extract a frame to use as a thumbnail.
        imagePath = `${filePath}.first-frame.png`;
        await extractFirstFrame(filePath, imagePath);
      }
      await generateThumbnail(imagePath, thumbnailPath);
      await setThumbnailForMedia(media, thumbnailPath);
    }
  }
}

try {
  await main();
} finally {
  db.close();
}
