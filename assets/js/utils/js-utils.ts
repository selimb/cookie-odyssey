export const jsUtils = {
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
};
