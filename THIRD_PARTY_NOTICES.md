# Third-party notices

Charge Deck is MIT licensed (see `LICENSE`). This file lists projects whose
work was used, and how.

## Code ported from MIT projects

The Razer Barracuda X provider (`src/providers/razer_barracuda.rs`) is a Rust
port of the protocol code in these MIT-licensed projects:

- **razer-barracuda-x-battery** — `src/Protocol.cs`
  https://github.com/mehdibouchami/razer-barracuda-x-battery
  Copyright (c) 2026 Mehdi Bouchami
- **razer-barracuda-battery-tray** — the 0x0552 (Macronix) framing
  https://github.com/KhromotozzDevOut/razer-barracuda-battery-tray
  Copyright (c) 2026 ArdorBatteryTray contributors

MIT License (applies to both projects above):

> Permission is hereby granted, free of charge, to any person obtaining a copy
> of this software and associated documentation files (the "Software"), to deal
> in the Software without restriction, including without limitation the rights
> to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
> copies of the Software, and to permit persons to whom the Software is
> furnished to do so, subject to the following conditions:
>
> The above copyright notice and this permission notice shall be included in all
> copies or substantial portions of the Software.
>
> THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
> IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
> FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
> AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
> LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
> OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
> SOFTWARE.

## Protocol facts only (no code copied)

Protocol facts (which byte means what, product IDs) are not copyrightable
code. We learned them from the projects below and wrote all code ourselves.

| Project | License | What we learned |
|---|---|---|
| G-Helper (seerge/g-helper) | GPL-3.0 | ASUS battery query ideas. **No code used.** |
| battery-tray (mikgangal/battery-tray) | no license file | ASUS battery query byte meanings. **No code used.** |
| OpenRazer | GPL-2.0 | Razer 90-byte report layout, command numbers, product IDs. **No code used.** |
| Solaar | GPL-2.0 | Logitech HID++ feature ideas. **No code used.** |
| libratbag | MIT | Logitech HID++ 2.0 documentation. No code used. |
| HeadsetControl | GPL-3.0 | Not used. |

## Rust crates

Charge Deck depends on these crates (all under permissive licenses such as
MIT / Apache-2.0; run `cargo license` for the full list of transitive
dependencies):

- `hidapi` (MIT) — USB HID access
- `windows` (MIT / Apache-2.0) — Microsoft's official Windows API bindings
- `eframe` / `egui` (MIT / Apache-2.0) — the window
- `winresource` (MIT, build-time only) — embeds the icon and version info into the `.exe`

## Installer tool

- **Inno Setup** (Jordan Russell and Martijn Laan; its own free license) — used at build time to create
  the setup wizard (`installer/charge-deck.iss`). Its license allows distributing the installers it
  creates. It is not part of the Charge Deck source or the app.

## Fonts

- **Rajdhani** (Bold, SemiBold) — bundled in `assets/fonts/` and compiled into the exe,
  because Charge Deck makes no network requests and cannot download fonts.
  Copyright (c) 2014, Indian Type Foundry. Licensed under the SIL Open Font
  License 1.1; the full text is in `assets/fonts/OFL.txt`.
  Source: https://github.com/google/fonts/tree/main/ofl/rajdhani

## Trademarks

Charge Deck is not affiliated with, endorsed by, or sponsored by ASUS, Razer,
Logitech or Microsoft. All product names are trademarks of their owners and are
used only to say which devices work.
