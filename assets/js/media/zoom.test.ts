import { describe, expect, test } from "bun:test";

import {
  clampScale,
  clampTransform,
  fitSize,
  IDENTITY,
  MAX_SCALE,
  MIN_SCALE,
  type Point,
  type Transform,
  zoomAt,
} from "./zoom";

// A portrait phone viewport showing a landscape image, which leaves black
// bars above and below.
const VIEW = { width: 400, height: 800 };
const FIT = { width: 400, height: 300 };

/** Maps a point of the unscaled element to the screen. */
function apply(t: Transform, p: Point): Point {
  return { x: t.x + p.x * t.scale, y: t.y + p.y * t.scale };
}

describe("fitSize", () => {
  test("fits a landscape image to the viewport width", () => {
    expect(fitSize({ width: 4000, height: 3000 }, VIEW)).toEqual(FIT);
  });

  test("fits a portrait image to the viewport height", () => {
    expect(fitSize({ width: 1000, height: 4000 }, VIEW)).toEqual({
      width: 200,
      height: 800,
    });
  });

  test("falls back to the viewport for an unknown size", () => {
    expect(fitSize({ width: 0, height: 0 }, VIEW)).toEqual(VIEW);
  });
});

describe("clampScale", () => {
  test("clamps to the allowed range", () => {
    expect(clampScale(0.5)).toBe(MIN_SCALE);
    expect(clampScale(2)).toBe(2);
    expect(clampScale(10)).toBe(MAX_SCALE);
  });
});

describe("clampTransform", () => {
  test("keeps the identity transform", () => {
    expect(clampTransform(IDENTITY, VIEW, FIT)).toEqual(IDENTITY);
  });

  test("stops the image edges at the viewport edges", () => {
    // At 2x the image is 800x600: wider than the viewport, shorter than it.
    const t = clampTransform({ scale: 2, x: 1000, y: 0 }, VIEW, FIT);
    expect(t.x).toBeCloseTo(0);
    const right = clampTransform({ scale: 2, x: -1000, y: 0 }, VIEW, FIT);
    // The right edge of the image (x = 400 unscaled) is at the viewport edge.
    expect(apply(right, { x: 400, y: 0 }).x).toBe(VIEW.width);
  });

  test("centers an axis on which the image is smaller than the viewport", () => {
    const t = clampTransform({ scale: 2, x: 0, y: 1000 }, VIEW, FIT);
    // The image center (y = 400 unscaled) stays at the viewport center.
    expect(apply(t, { x: 0, y: 400 }).y).toBe(VIEW.height / 2);
  });

  test("stops the image edges past the black bars when taller than the viewport", () => {
    // At 4x the image is 1600x1200: taller than the viewport.
    const top = clampTransform({ scale: 4, x: 0, y: 1000 }, VIEW, FIT);
    // The top edge of the image (y = 250 unscaled) is at the viewport top.
    expect(apply(top, { x: 0, y: 250 }).y).toBe(0);
    const bottom = clampTransform({ scale: 4, x: 0, y: -10_000 }, VIEW, FIT);
    expect(apply(bottom, { x: 0, y: 550 }).y).toBe(VIEW.height);
  });
});

describe("zoomAt", () => {
  test("keeps the zoom point fixed on screen", () => {
    const point = { x: 100, y: 300 };
    const start: Transform = { scale: 1.5, x: -20, y: -40 };
    const t = zoomAt(start, 3, point);
    expect(t.scale).toBe(3);
    // The content under `point` before zooming is still under it after.
    const content = {
      x: (point.x - start.x) / start.scale,
      y: (point.y - start.y) / start.scale,
    };
    expect(apply(t, content)).toEqual(point);
  });
});
