# Security policy

## Supported versions

Security fixes go into the latest release. LampRabit updates itself, so make sure you're on the newest version (Settings → Updates).

## Reporting a vulnerability

Please **don't** open a public issue for security problems. Instead, report it privately through GitHub: [Report a vulnerability](https://github.com/LampRabit/LampRabit/security/advisories/new).

Include what you found, how to reproduce it, and which version and OS you used. You'll get a reply as soon as possible, and credit in the release notes if you'd like.

## Scope

LampRabit is a **local development** environment. The servers it runs (Apache, Nginx, PHP, MariaDB with an empty root password, Mailpit, Redis) are configured for convenience on your own machine, not for exposure to the internet. Shared sites (Cloudflare Tunnel, ngrok) are public while sharing is on; stop sharing when you're done.
