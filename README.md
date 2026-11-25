<div align="center">

<img src="docs/images/logo.png" width="112" alt="Omni-Route icon: two Wi-Fi signals facing each other">

# Omni-Route

### All your connections. One faster, more resilient route.

**A free, source-available multi-link internet app for Windows 10 and 11.**

Ethernet, Wi-Fi, and USB tethering work together to balance your PC's traffic and accelerate supported downloads.

[Download for Windows](https://github.com/mr-peppermint/Omni-Route/releases/latest) · [How it works](#how-it-works) · [Build from source](#build-from-source) · [Italiano](README.it.md)

<br>

[![Windows 10 and 11](https://img.shields.io/badge/Windows-10%20%7C%2011-24314A?style=flat-square&logo=windows&logoColor=white)](https://github.com/mr-peppermint/Omni-Route)
[![Release v1.0.0](https://img.shields.io/badge/Release-v1.0.0-6A63D8?style=flat-square)](https://github.com/mr-peppermint/Omni-Route/releases)
[![PolyForm Shield 1.0.0](https://img.shields.io/badge/License-PolyForm%20Shield-24314A?style=flat-square)](LICENSE)

</div>

---

## Put every link to work

Windows usually sends traffic through one network adapter at a time. Omni-Route balances active connections across Ethernet, Wi-Fi, and USB cellular adapters, adapting as links slow down or drop.

It also splits supported large downloads into parallel segments and fetches them across multiple links. No account, cloud relay, or remote server is required.

| System-wide balancing | Download acceleration | Stay connected |
| --- | --- | --- |
| Distributes active Windows traffic flows across available adapters. | Retrieves file segments in parallel over separate network paths when the server supports it. | Continues balancing new traffic over surviving links when a connection drops. |

Other features include live per-link speed gauges, metered-link protection, and a native Windows background service with system tray controls.

## How it works

1. **Connect your links.** Use Wi-Fi, Ethernet, or a phone connected by USB tethering.
2. **Turn balancing on.** Open Omni-Route and switch on system-wide balancing.
3. **Use your PC as usual.** Traffic flows across the available links; compatible downloads can use parallel segments.

## Install

### Quick install

Open PowerShell and run:

```powershell
powershell -ep bypass -c "irm https://raw.githubusercontent.com/mr-peppermint/Omni-Route/main/install.ps1|iex"
```

### Installer

Download **`Omni-Route_1.0.0_x64-setup.exe`** from [GitHub Releases](https://github.com/mr-peppermint/Omni-Route/releases/latest).

## Build from source

### Requirements

- [Rust](https://rustup.rs/) 1.77.2 or newer
- [Node.js](https://nodejs.org/) 18 or newer
- Visual Studio C++ Build Tools and Windows SDK

```bash
git clone https://github.com/mr-peppermint/Omni-Route.git
cd Omni-Route
npm install
npm run tauri dev
```

Create the production installer with `npm run tauri build`.

## License and trademark

Omni-Route is available under the [PolyForm Shield License 1.0.0](LICENSE) for personal, academic, and internal workplace use. Commercial competing products require a separate commercial license; see [COMMERCIAL.md](COMMERCIAL.md).

**Omni-Route** and its logo are trademarks owned by [mr-peppermint](https://github.com/mr-peppermint). See [TRADEMARK.md](TRADEMARK.md). Third-party component notices, including the Wintun driver, are in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
