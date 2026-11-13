import { create } from "zustand";

/** "system" follows the OS reduce-motion setting; "reduced" keeps fades but drops movement. */
export type Motion = "system" | "full" | "reduced" | "off";

// jsdom (component tests) has no matchMedia: treat it as no preference.
const reduceQuery = () => window.matchMedia?.("(prefers-reduced-motion: reduce)") ?? ({ matches: false, addEventListener() {}, removeEventListener() {} } as unknown as MediaQueryList);

function readMotion(): Motion {
  try {
    const m = localStorage.getItem("motion");
    if (m === "full" || m === "reduced" || m === "off") return m;
  } catch {}
  return "system";
}

/** Stamps the resolved level on <html data-motion>, which the CSS keys off. */
function apply(motion: Motion) {
  document.documentElement.dataset.motion = motion === "system" ? (reduceQuery().matches ? "reduced" : "full") : motion;
}

export const useMotion = create<{ motion: Motion; setMotion: (m: Motion) => void }>((set) => ({
  motion: readMotion(),
  setMotion: (motion) => {
    try {
      if (motion === "system") localStorage.removeItem("motion");
      else localStorage.setItem("motion", motion);
    } catch {}
    apply(motion);
    set({ motion });
  },
}));

apply(useMotion.getState().motion);
reduceQuery().addEventListener("change", () => apply(useMotion.getState().motion));

/** Whether decorative animation (like the logo's idle loop) should run right now. */
export const motionAllowed = () => document.documentElement.dataset.motion === "full";
