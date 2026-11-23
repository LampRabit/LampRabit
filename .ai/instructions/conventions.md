# Code conventions

- Comments explain *why*, in plain sentences; match the density of the surrounding code. Error messages tell the user what to do next.
- UI colors come from the theme tokens in `src/index.css` (`bg-surface`, `text-muted`, `accent`…), never fixed hex, so dark mode works. Check both themes. Follow the visual style in [ux-principles](../context/decisions/ux-principles.md#visual-style-the-users-choices-2026-11-23): neutral icons, colour only for status, sentence case, weights 400–500.
- **Keyboard**: every shortcut is listed in `SHORTCUTS` in `src/lib/shortcuts.ts` (shown by the `?` dialog); handlers use `useHotkeys`, which pauses while typing or while a dialog or menu is open. Add new shortcuts to both.
- Runtime cards on Services use the shared pieces in `components/versions.tsx` (in-use tile, installed tiles, download list); Node.js uses them too.
- Dialogs use `components/Dialog.tsx` (focus in, Tab trapped, Escape closes, focus restored). Mark the field to focus with `data-autofocus`; mount stateful dialogs only while open.
- OS-specific behaviour sits behind `cfg!`/`#[cfg]`; where a feature can't work on a platform, the UI says why instead of hiding it.
- Version bumps touch `package.json`, `src-tauri/tauri.conf.json` and `src-tauri/Cargo.toml` together.
