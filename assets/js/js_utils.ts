const onceRegistry = new Set();

// XXX still need this?
export const jsUtils = {
  once: (key: string, fn: () => void) => {
    if (onceRegistry.has(key)) {
      return;
    }
    onceRegistry.add(key);
    fn();
  },
  show($elem: Element) {
    const cl = $elem.classList;
    if (cl.contains("hidden")) {
      cl.remove("hidden");
    }
  },
  hide($elem: Element) {
    const cl = $elem.classList;
    if (!cl.contains("hidden")) {
      cl.add("hidden");
    }
  },
  observe<T extends Element>(selector: string, fn: ($elem: T) => void) {
    const onMutation: MutationCallback = (mutations) => {
      for (const mut of mutations) {
        for (const node of mut.addedNodes) {
          if (node instanceof Element) {
            if (node.matches(selector)) {
              fn(node as T);
            } else {
              for (const subnode of node.querySelectorAll(selector)) {
                fn(subnode as T);
              }
            }
          }
        }
      }
    };
    const observer = new MutationObserver(onMutation);
    observer.observe(document, { childList: true, subtree: true });
  },
};
