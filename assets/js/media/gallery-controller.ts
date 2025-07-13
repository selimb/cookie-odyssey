import lightGallery from "lightgallery";
import type { LightGallery } from "lightgallery/lightgallery";
import lgThumbnail from "lightgallery/plugins/thumbnail";
import lgVideo from "lightgallery/plugins/video";
import lgZoom from "lightgallery/plugins/zoom";

import { TypedController } from "../utils/stimulus-typed";

export class JournalEntryMediaGalleryController extends TypedController(
  "media--gallery",
  "element",
  // Don't really need this to be a target, but this feels more kosher.
  { targets: { item: "a" } },
) {
  gallery: LightGallery | undefined;

  connect(): void {
    this.gallery = lightGallery(this.element, {
      plugins: [lgZoom, lgThumbnail, lgVideo],
      supportLegacyBrowser: false,
      speed: 100,
      // Matches `targets#item` above.
      selector: "[data-media--gallery-target=item]",
      enableDrag: false,
      enableThumbDrag: false,
      // XXX
      hideScrollbar: true,
    });
  }

  disconnect(): void {
    if (this.gallery) {
      this.gallery.destroy();
    }
  }
}
