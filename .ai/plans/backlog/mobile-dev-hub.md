# Mobile Dev Hub, Dart and Flutter, phone ↔ local backend

Home: Devices (new) + Services (Flutter/Dart/JDK runtimes) + Projects. Source: [spec](../source-specs/2026-11-21-feature-context.md) "Feature 3" and "Feature 4". Status: **not started**. Size: XL; build as an opt-in module ("Add mobile development").

Positioning: not an IDE or debugger. LampRabit owns the environment around the app: SDKs, devices, backend services, and the phone reaching the local backend.

## Already in LampRabit

Java runtime with `.java-version`, Gradle/Maven, Node (NVM, corepack), the project doctor, processes with ports, Share tunnels (Cloudflare/ngrok), port routing.

## Feasibility (checked against the code)

- **Flutter SDK:** Google publishes `releases_<os>.json` with archive URLs and SHA-256, so it can be a checked runtime (large, ~1 GB). Respect FVM (`.fvmrc`) and `pubspec.yaml` constraints instead of a parallel scheme. Standalone Dart SDK for backend/CLI projects.
- **Android SDK / Xcode:** detect, don't bundle (multi-GB). `platform-tools` (adb) alone is small and could be a runtime.
- **Hot reload** (`r`/`R` on stdin): the process runner must keep stdin open and offer "send key". This is new work in `apps.rs`; `flutter run --machine` (JSON-RPC) is the richer alternative later.
- **`adb reverse`** per endpoint is the core trick: the phone reaches `http://localhost:<port>` with no LAN IP, firewall, DNS or certificate trust. Feasible; re-apply on reconnect.
- **LAN testing** (phone on Wi-Fi without adb) needs a web server listening on the LAN interface. Today everything binds 127.0.0.1 on purpose, so this must be an explicit, visible opt-in per endpoint with a firewall rule prompt. Android 7+ apps don't trust user CAs, so prefer adb reverse (http localhost) or a Share tunnel (publicly trusted HTTPS).
- **iOS:** simulator reaches localhost directly; physical devices need macOS + Xcode, so detection and guidance only in v1.

## Plan

1. Runtimes: Flutter (bundles Dart), Dart, adb/platform-tools; JDK pinning is already there (doctor checks JDK vs Gradle/AGP).
2. Detection: `pubspec.yaml` with `flutter:` → platform folders decide kinds (`web/` → Web endpoint, `windows/macos/linux` → Desktop, `android/ios` → Mobile); Dart with `shelf`/`dart_frog`/`serverpod` → API; `executables:`/`bin/` → CLI; else Package. Serverpod → offer Postgres + Redis.
3. Create apps by wrapping official CLIs only: `flutter create`, `npx create-expo-app@latest`, `npx @react-native-community/cli init`.
4. Unified mobile doctor: `flutter doctor -v`, `npx expo-doctor`, `npx react-native doctor` + own checks (JAVA_HOME, ANDROID_HOME, platform-tools on PATH, several adb binaries, Node version, CocoaPods on macOS), each with a fix.
5. Devices home: `adb devices -l`, wireless pairing (`adb pair`, `adb mdns services`, QR), `flutter devices --machine`, emulators list/launch; APK install, logcat filtered by package, screenshot; scrcpy optional.
6. "Run on device": pick device → `adb reverse` for every endpoint (Metro 8081, API ports) → start the dev server → launch (`flutter run -d`, `npx expo start`, `npx react-native run-android`).
7. Per endpoint, show the URL for each target: desktop `https://x.test`, Android emulator `http://10.0.2.2:<port>`, iOS simulator `http://127.0.0.1:<port>`, phone via adb reverse, LAN `http://<ip>:<port>` + QR, tunnel URL; optional `--dart-define=API_BASE_URL=…`.

Out of scope: debugger, widget inspector, store builds/signing, cloud builds (EAS).
