# In-house media viewer

The full-screen viewer for the images and videos of a Day or Getaway page is our own Stimulus controller, not a gallery library.
Swiping between slides is native CSS scroll snapping, videos are plain `<video>` elements, and zoom is a small hand-written pinch and double-tap on a single image.
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

- Zoom is basic: no momentum or rubber-banding, and no swiping to the next slide while zoomed.
  Native-quality zoom would mean rewriting PhotoSwipe.
- Gesture bugs are ours, and iOS Safari quirks have to be tested on a real phone.
- The viewer is always black, whatever the theme, so its controls are custom styles rather than DaisyUI components.
- Closing the viewer with the back button has to coexist with HTMX Boost's own history handling.
