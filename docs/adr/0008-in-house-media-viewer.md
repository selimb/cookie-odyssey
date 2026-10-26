# In-house media viewer

The full-screen viewer for the images and videos of a Day or Getaway page is our own Stimulus controller, not a gallery library.
Swiping between slides is native CSS scroll snapping, videos are plain `<video>` elements, and zoom on touch screens is the browser's own pinch zoom.
Most users are on phones, and no library gave a good mobile experience for a journal with lots of videos.

## Considered Options

- **lightGallery**, which we used first.
  Pinch zoom was janky and the zoom button useless on a phone.
  The toolbar and thumbnail strip took about a third of the screen, which was obvious when casting a phone to a TV.
  Captions landed inside or outside the image depending on which slide was opened first, because lightGallery measures the caption height once, on the first slide, by design.
  Most of this cannot be fixed with settings.
- **PhotoSwipe**, which has the best touch handling (pinch, momentum, swipe to close).
  Rejected because video and captions are not built in, and our Journals have many videos.
  Its video and caption plugins exist, but video inside a JS-driven swipe gallery is where gesture handling fights with native controls.
- **In-house**, chosen.
  Letting the browser do the swiping gives native scroll physics for free, and leaves video controls untouched since nothing intercepts their touches.
  Captions are ordinary HTML in each slide, so they are placed the same way on every slide.

## Consequences

- Zoom on touch screens is the browser's pinch zoom of the whole page, not a zoom of the image.
  A hand-written pinch zoom (CSS transform driven by touch events) was tried first and dropped: it still felt janky next to native apps, and the browser's own zoom felt better even on the page's thumbnails.
  The viewer locks swiping and hides its controls while the page is zoomed, and there is no double-tap zoom.
  There is no API to reset the page zoom, so closing the viewer while zoomed leaves the page zoomed.
- With a mouse, double-click zooms the image and dragging pans it, with a CSS transform.
- Gesture bugs are ours, and iOS Safari quirks have to be tested on a real phone.
- The viewer is always black, whatever the theme, so its controls are custom styles rather than DaisyUI components.
- Opening the viewer does not add a history entry, like other blog galleries.
  The Android back gesture still closes it, since browsers close a modal `<dialog>` on it, but elsewhere (e.g. an iOS edge swipe) Back leaves the page.
  Adding an entry was tried and dropped: it had to work around HTMX Boost's own history handling, and left a dead entry for the Forward button.
