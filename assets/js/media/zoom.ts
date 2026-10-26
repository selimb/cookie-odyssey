/**
 * [gallery-zoom] Pure math for the gallery image zoom with a mouse.
 *
 * All transforms are expressed as `translate(x, y) scale(scale)` with the
 * transform origin at the top-left corner of the zoomed element, and the
 * element is assumed to fill the viewport with the image centered inside it.
 * See [gallery-media-fit].
 */

export type Size = { width: number; height: number };
export type Point = { x: number; y: number };
export type Transform = { scale: number; x: number; y: number };

export const MIN_SCALE = 1;
export const MAX_SCALE = 4;

export const IDENTITY: Transform = { scale: 1, x: 0, y: 0 };

/** Returns the size of the largest `natural`-shaped box that fits in `view`. */
export function fitSize(natural: Size, view: Size): Size {
  if (natural.width <= 0 || natural.height <= 0) return view;
  const ratio = Math.min(
    view.width / natural.width,
    view.height / natural.height,
  );
  return { width: natural.width * ratio, height: natural.height * ratio };
}

/** Clamps `scale` to the allowed range. */
export function clampScale(scale: number): number {
  return Math.min(MAX_SCALE, Math.max(MIN_SCALE, scale));
}

/**
 * Clamps the translation so that the visible image (of size `fit`, centered
 * in `view`) never leaves the viewport.
 * An axis on which the scaled image is smaller than the viewport is centered.
 */
export function clampTransform(t: Transform, view: Size, fit: Size): Transform {
  return {
    scale: t.scale,
    x: clampAxis(t.x, t.scale, view.width, fit.width),
    y: clampAxis(t.y, t.scale, view.height, fit.height),
  };
}

function clampAxis(
  offset: number,
  scale: number,
  viewLen: number,
  fitLen: number,
): number {
  // Position of the image inside the (unscaled) element.
  const start = (viewLen - fitLen) / 2;
  const scaledLen = fitLen * scale;
  if (scaledLen <= viewLen) {
    // Center.
    return viewLen / 2 - scale * (start + fitLen / 2);
  }
  // Leading edge must not move past 0, trailing edge must not move before
  // `viewLen`.
  const max = -scale * start;
  const min = viewLen - scale * (start + fitLen);
  return Math.min(max, Math.max(min, offset));
}

/** Returns `t` rescaled to `scale` such that `point` stays fixed on screen. */
export function zoomAt(t: Transform, scale: number, point: Point): Transform {
  const ratio = scale / t.scale;
  return {
    scale,
    x: point.x - (point.x - t.x) * ratio,
    y: point.y - (point.y - t.y) * ratio,
  };
}
