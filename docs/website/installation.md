# Installing LampRabit

> **Status:** installers for every platform are on the GitHub Releases page: https://github.com/Shivam-SV/LampRabit/releases/latest. Windows and Linux are supported; macOS is in preview. Runtime support varies by OS and architecture (see the table and the in-app Services catalog).

## System requirements

| | Windows | macOS | Linux |
|---|---|---|---|
| OS version | Windows 10 (1809 or later) or Windows 11, 64-bit | macOS 11 Big Sur or later | Ubuntu 22.04+, Debian 12+, Fedora 38+ or similar |
| Web view | Microsoft Edge WebView2 (built into Windows 11; the installer adds it on Windows 10) | Built in | WebKitGTK 4.1 |
| Runtime libraries | Microsoft Visual C++ 2015–2022 Redistributable (x64), needed by Apache and PHP | — | — |
| Disk space | ~150 MB for the app and a basic stack; more for extra versions and databases | | |

### What can be downloaded on each platform

| Runtime | Windows | macOS | Linux |
|---|---|---|---|
| Apache | Yes | Preview (LampRabit's builds) | Yes (LampRabit's builds) |
| Nginx | Yes | Preview (LampRabit's builds) | Yes (LampRabit's builds) |
| PHP | Yes (8.1–8.5) | Preview (8.1–8.5, LampRabit's static builds) | Yes (8.1–8.5, LampRabit's static builds) |
| MariaDB | Yes | Preview (LampRabit's builds) | Yes |
| MySQL | Yes | Preview (verify current release coverage) | Yes (needs the system's libaio and libnuma; LampRabit names the packages if they're missing) |
| Composer | Yes | Yes | Yes |
| Redis (with redis-cli) | Yes (community redis-windows build) | Yes (LampRabit's builds) | Yes (LampRabit's builds) |
| Memcached | Yes (nono303's build) | Preview (LampRabit's builds) | Yes (LampRabit's builds) |
| Mailpit, phpMyAdmin, Adminer | Yes | Yes | Yes |

## Install

### Windows
1. Download `LampRabit_<version>_x64-setup.exe` (or the `.msi` for managed installs) from the Releases page.
2. Run it. LampRabit installs per user; administrator rights aren't required for the app itself.
3. **"Windows protected your PC"** appears because the installer isn't code-signed yet. Click **More info**, then **Run anyway**.
4. Apache and PHP need the Microsoft Visual C++ 2015–2022 Redistributable (x64). LampRabit checks for it and shows an **Install** button on the Overview page if it's missing.

### macOS
1. Download `LampRabit_<version>_aarch64.dmg` (Apple Silicon: M1 and later) or `LampRabit_<version>_x64.dmg` (Intel).
2. Open it and drag LampRabit to Applications.
3. Open LampRabit. Because it isn't signed yet, macOS says it can't be opened: go to **System Settings → Privacy & Security**, scroll down and click **Open Anyway**, then confirm. (Or right-click the app in Applications and choose **Open**.) You only need to do this once.

### Linux
- **AppImage:** download `LampRabit_<version>_amd64.AppImage`, make it executable (`chmod +x LampRabit_*.AppImage`) and run it.
- **Debian/Ubuntu:** `sudo apt install ./lampRabit_<version>_amd64.deb`

## First run

1. **Environment folder.** LampRabit keeps everything in one folder: `C:\lampRabit` on Windows, `~/lampRabit` on macOS and Linux. You can change it in Settings → General. Inside it:

   ```
   bin/    downloaded runtimes (Apache, PHP, MariaDB, ...)
   etc/    generated configuration
   www/    your projects: one folder per site
   data/   databases
   logs/   all log files
   ssl/    local certificate authority and site certificates
   tmp/    sessions, uploads, temporary files
   ```

2. **Install your stack.** Open **Services** and install Apache (or Nginx), PHP and MariaDB. The first version you install of each becomes the active one.
3. **Start.** Click **Start all** in the title bar.
4. **Create a site.** Click **Open www**, create a folder such as `hello`, and add `index.php`:

   ```php
   <?php phpinfo();
   ```

5. **Add it to the hosts file.** On the Sites page, click **Apply changes** (or restart Apache). Windows asks for administrator approval once; approve it. From then on the hosts file is updated automatically when Apache starts and the site list has changed.
6. **Trust the certificate.** On the Sites page, click **Trust certificate** and confirm. Restart your browser if it was open.
7. Open `https://hello.test`.

### Optional
- **Developer tools:** Services → Developer tools → install phpMyAdmin, Adminer and Mailpit.
- **No hosts file:** set the domain suffix to `localhost` in Settings. Sites become `hello.localhost` and browsers resolve them without the hosts file.

## Firefox and HTTPS

Firefox keeps its own certificate list. To make it trust LampRabit's certificates, open `about:config`, set `security.enterprise_roots.enabled` to `true`, and restart Firefox. Chrome, Edge, Brave and Safari use the system certificate store and work after **Trust certificate**.

On Linux, **Trust certificate** asks for your password once and adds the certificate to the system store. Chrome, Chromium and Firefox keep their own lists, which LampRabit updates too when `certutil` is installed (`sudo apt install libnss3-tools`, or `nss-tools` on Fedora).

On Linux, normal users can't listen on ports below 1024. The first time Apache or Nginx starts on port 80/443, LampRabit asks for your password once and saves the change in `/etc/sysctl.d/50-lamprabit.conf`. To avoid it, use ports above 1024 (such as 8080 and 8443) in Settings → Ports.

## Ports used

| Service | Default port | Change in |
|---|---|---|
| Apache (or Nginx alone) | 80, 443 | Settings → Ports |
| Nginx next to Apache | 8080, 8443 | Settings → Ports |
| MariaDB | 3306 | Settings → Ports |
| PHP FastCGI workers | 9000 and up (10 per PHP version) | Settings → Ports |
| Mailpit SMTP / inbox | 1025 / 8025 | Settings → Email testing |

## Uninstall

Uninstalling removes everything LampRabit set up and **keeps your projects (`www/`) and databases (`data/`)**, so a later install picks them up again. It deletes the runtimes, configs, logs and certificates in the environment folder (`bin`, `etc`, `logs`, `tmp`, `ssl`, `shims`; nothing else in that folder is touched), LampRabit's settings, the `.test` entries in the hosts file, the trusted "lampRabit Local CA" certificate, LampRabit's commands on your PATH, the saved ngrok token and starting with your computer.

- **Windows:** Settings → Apps → LampRabit → Uninstall. The uninstaller asks you to close LampRabit, then does the cleanup; Windows asks for confirmation for the hosts file and the certificate. (Updating LampRabit never runs this.)
- **macOS and Linux:** in LampRabit, open Settings → Troubleshooting → **Remove LampRabit from this computer**. It shows what is deleted and kept before doing anything, then quits. Afterwards move LampRabit to the Trash (macOS), or delete the AppImage / run `sudo apt remove lamprabit` (Linux). On Linux your password is asked once, for the hosts file, the certificate and the ports 80/443 setting.
- The same Settings action works on Windows too, if you'd rather clean up before uninstalling.

To delete your projects and databases as well, delete the environment folder (`C:\lampRabit` or `~/lampRabit`) afterwards. Back up `www/` and `data/` first if you might need them.

If something couldn't be undone (for example a cancelled administrator prompt), LampRabit lists it with what to do by hand.

## Building from source

For contributors and early testers.

1. Install [Node.js](https://nodejs.org) 20 or later and [Rust](https://rustup.rs) (stable).
2. Platform prerequisites for Tauri:
   - Windows: Visual Studio 2022 Build Tools with the "Desktop development with C++" workload; WebView2 (built into Windows 11).
   - macOS: Xcode Command Line Tools (`xcode-select --install`).
   - Linux: WebKitGTK 4.1 and build tools, see https://tauri.app/start/prerequisites/
3. Clone and run:

   ```bash
   git clone https://github.com/Shivam-SV/LampRabit.git
   cd LampRabit
   npm install
   npm run tauri dev
   ```

4. Build an installer for your platform: `npm run tauri build`.
