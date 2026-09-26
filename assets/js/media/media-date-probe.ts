/* eslint-disable unicorn/number-literal-case -- Prettier lowercases hex literals, which this rule rejects; this file is full of byte markers. */

// Diagnostic helpers for inspecting the date metadata a browser exposes for a
// dropped or picked file.
// This exists to answer one question before we commit to a design: for a given
// device and import path (Apple Photos drag, iPhone picker, ...), does
// `file.lastModified` reflect the capture date, and is the real capture date
// recoverable from EXIF or the video container without a third-party library?
// Used only by the thumbnail demo page; not part of the real upload flow.

export type MediaDateProbe = {
  // Detected container format, from magic bytes.
  format: string;
  // EXIF DateTimeOriginal (tag 0x9003) for JPEG, if present.
  exifDateTimeOriginal: string | null;
  // ISO BMFF (MP4/MOV) movie-header creation time as ISO string, if present.
  videoCreationTime: string | null;
};

export async function probeMediaDate(file: File): Promise<MediaDateProbe> {
  const headBytes = new Uint8Array(await file.slice(0, 16).arrayBuffer());
  const format = detectFormat(headBytes);

  let exifDateTimeOriginal: string | null = null;
  if (format === "JPEG") {
    // EXIF lives in the APP1 segment near the start, so a small head slice is
    // more than enough.
    const head = await file.slice(0, 256 * 1024).arrayBuffer();
    exifDateTimeOriginal = readExifDateTimeOriginal(head);
  }

  let videoCreationTime: string | null = null;
  if (format.startsWith("ISOBMFF")) {
    videoCreationTime = await readVideoCreationTime(file);
  }

  return { format, exifDateTimeOriginal, videoCreationTime };
}

function detectFormat(bytes: Uint8Array): string {
  const ascii = (start: number, len: number): string =>
    String.fromCodePoint(...bytes.subarray(start, start + len));

  if (bytes[0] === 0xff && bytes[1] === 0xd8 && bytes[2] === 0xff) {
    return "JPEG";
  }
  if (bytes[0] === 0x89 && ascii(1, 3) === "PNG") {
    return "PNG";
  }
  if (ascii(0, 3) === "GIF") {
    return "GIF";
  }
  if (ascii(0, 4) === "RIFF" && ascii(8, 4) === "WEBP") {
    return "WebP";
  }
  // MP4/MOV/HEIC all share the ISO BMFF "ftyp" box. The major brand tells us
  // which one (heic/heix/mif1 = HEIC, qt = MOV, isom/mp42 = MP4, ...).
  if (ascii(4, 4) === "ftyp") {
    const brand = ascii(8, 4).trim();
    return `ISOBMFF (ftyp '${brand}')`;
  }
  return "unknown";
}

function readExifDateTimeOriginal(buf: ArrayBuffer): string | null {
  try {
    const view = new DataView(buf);
    // SOI marker.
    if (view.getUint16(0) !== 0xff_d8) {
      return null;
    }

    let offset = 2;
    while (offset + 4 <= view.byteLength) {
      const marker = view.getUint16(offset);
      // Every JPEG marker starts with 0xFF.
      if ((marker & 0xff_00) !== 0xff_00) {
        return null;
      }
      const size = view.getUint16(offset + 2);
      // APP1 segment carries the EXIF block.
      if (marker === 0xff_e1) {
        const exifStart = offset + 4;
        const isExif =
          view.getUint32(exifStart) === 0x45_78_69_66 && // "Exif"
          view.getUint16(exifStart + 4) === 0x00_00;
        if (isExif) {
          return parseTiffForDateTimeOriginal(view, exifStart + 6);
        }
      }
      // SOS marks the start of image data; no metadata past here.
      if (marker === 0xff_da) {
        return null;
      }
      offset += 2 + size;
    }
    return null;
  } catch {
    return null;
  }
}

function parseTiffForDateTimeOriginal(
  view: DataView,
  tiffStart: number,
): string | null {
  const byteOrder = view.getUint16(tiffStart);
  const little = byteOrder === 0x49_49; // "II"
  if (!little && byteOrder !== 0x4d_4d) {
    // Neither "II" nor "MM".
    return null;
  }

  const u16 = (o: number): number => view.getUint16(o, little);
  const u32 = (o: number): number => view.getUint32(o, little);

  // TIFF magic.
  if (u16(tiffStart + 2) !== 0x00_2a) {
    return null;
  }

  // Find an entry by tag within an IFD. Returns the 12-byte entry offset.
  const findEntry = (ifdOffset: number, tag: number): number | null => {
    const count = u16(ifdOffset);
    for (let i = 0; i < count; i++) {
      const entry = ifdOffset + 2 + i * 12;
      if (u16(entry) === tag) {
        return entry;
      }
    }
    return null;
  };

  const ifd0 = tiffStart + u32(tiffStart + 4);
  // Tag 0x8769 (ExifIFDPointer) points at the sub-IFD holding capture tags.
  const exifPtrEntry = findEntry(ifd0, 0x87_69);
  if (exifPtrEntry === null) {
    return null;
  }
  const exifIfd = tiffStart + u32(exifPtrEntry + 8);

  // Tag 0x9003 (DateTimeOriginal), ASCII "YYYY:MM:DD HH:MM:SS".
  const dtEntry = findEntry(exifIfd, 0x90_03);
  if (dtEntry === null) {
    return null;
  }

  const valueCount = u32(dtEntry + 4);
  const valueOffset = tiffStart + u32(dtEntry + 8);
  let s = "";
  for (let i = 0; i < valueCount && i < 32; i++) {
    const c = view.getUint8(valueOffset + i);
    if (c === 0) {
      break;
    }
    s += String.fromCodePoint(c);
  }
  return s || null;
}

type Box = { dataStart: number; end: number };

async function readVideoCreationTime(file: File): Promise<string | null> {
  try {
    // Find the top-level 'moov' box, then its 'mvhd' child, reading only the
    // slices we need so we never load a whole video into memory.
    const moov = await findBox(file, 0, file.size, "moov");
    if (!moov) {
      return null;
    }
    const mvhd = await findBox(file, moov.dataStart, moov.end, "mvhd");
    if (!mvhd) {
      return null;
    }

    const header = new DataView(
      await file.slice(mvhd.dataStart, mvhd.dataStart + 20).arrayBuffer(),
    );
    const version = header.getUint8(0);
    // creation_time follows the 1-byte version and 3-byte flags.
    const creation =
      version === 1 ? Number(header.getBigUint64(4)) : header.getUint32(4);
    // ISO BMFF epoch is 1904-01-01 UTC; convert to the Unix epoch.
    const unixSeconds = creation - 2_082_844_800;
    if (unixSeconds <= 0) {
      return null;
    }
    return new Date(unixSeconds * 1000).toISOString();
  } catch {
    return null;
  }
}

async function findBox(
  file: File,
  start: number,
  end: number,
  type: string,
): Promise<Box | null> {
  let offset = start;
  while (offset + 8 <= end) {
    const head = new DataView(
      await file.slice(offset, offset + 16).arrayBuffer(),
    );
    let size = head.getUint32(0);
    const boxType = String.fromCodePoint(
      head.getUint8(4),
      head.getUint8(5),
      head.getUint8(6),
      head.getUint8(7),
    );
    let headerSize = 8;
    if (size === 1) {
      // 64-bit largesize.
      size = Number(head.getBigUint64(8));
      headerSize = 16;
    } else if (size === 0) {
      // Box extends to the end of the file.
      size = end - offset;
    }
    if (boxType === type) {
      return { dataStart: offset + headerSize, end: offset + size };
    }
    if (size <= 0) {
      return null;
    }
    offset += size;
  }
  return null;
}
