import { Keyboard } from "lucide-react";
import { SHORTCUTS } from "../lib/shortcuts";
import { useAppStore } from "../lib/store";
import Dialog, { Kbd } from "./Dialog";

/** The "?" dialog: every keyboard shortcut, grouped by where it works. */
export default function ShortcutsDialog() {
  const open = useAppStore((s) => s.shortcutsOpen);
  const setOpen = useAppStore((s) => s.setShortcutsOpen);

  return (
    <Dialog open={open} onClose={() => setOpen(false)} title="Keyboard shortcuts" icon={<Keyboard size={20} />} width="max-w-2xl">
      <div className="grid gap-8 sm:grid-cols-2">
        {SHORTCUTS.map((group) => (
          <section key={group.group}>
            <h3 className="mb-2 text-xs font-medium text-muted">{group.group}</h3>
            <dl className="divide-y divide-line">
              {group.items.map((item) => (
                <div key={item.label} className="flex items-center justify-between gap-4 py-2 text-sm">
                  <dt>{item.label}</dt>
                  <dd className="flex shrink-0 items-center gap-1">
                    {item.keys.map((k, i) => (
                      <span key={k} className="flex items-center gap-1">
                        {i > 0 && <span className="text-xs text-muted">{item.keys[0] === "Shift" ? "+" : "or"}</span>}
                        <Kbd>{k}</Kbd>
                      </span>
                    ))}
                  </dd>
                </div>
              ))}
            </dl>
          </section>
        ))}
      </div>
      <p className="mt-6 text-sm text-muted">
        Shortcuts pause while you type in a field. Tab and Shift+Tab move through every control, and Escape closes dialogs and menus.
      </p>
    </Dialog>
  );
}
