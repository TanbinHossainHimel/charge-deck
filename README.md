# Charge Deck

*All your wireless gear's battery on one deck — no Armoury Crate, Synapse or G HUB needed.*

Charge Deck is a small Windows 10/11 app that finds your wireless keyboards,
mice, headsets and controllers and shows every battery level in one window.
Lowest battery first, so you see what needs charging at a glance.

> 📷 *Screenshot placeholder — add `docs/screenshot.png` here.*

- One card per device: icon, name, battery %, colour bar (green > 50 %, yellow 20–50 %, red < 20 %), source and status.
- `≈` in front of a % means "estimated" (from voltage or a few bars). The voltage is shown too.
- Devices that disappear stay on the list, greyed out, with "Last seen HH:MM".
- Refreshes every 5 minutes, plus a **Refresh** button. Optional **Always on top**.
- Runs as a normal user. No driver, no service, no network.

## Download and install

1. Download **`ChargeDeck-Setup-<version>.exe`** from the project's **Releases** page.
2. Double-click it and click **Next** (three times), then **Install** and **Finish**.

That's it: no admin rights, no driver, no service. Windows may show a blue "Windows protected your PC"
box because the installer isn't code-signed yet: click **More info → Run anyway**.
Full guide, uninstalling and troubleshooting: **[INSTALL.md](INSTALL.md)**.
(Prefer to build it yourself? That's in INSTALL.md too. Maintainers: see [RELEASING.md](RELEASING.md).)

## The honest truth: there is no universal way

Every brand talks to its 2.4 GHz dongle with its own private protocol. So
Charge Deck works in **three layers**:

1. **Things Windows already knows** — Bluetooth devices and game controllers
   report their battery through Windows itself. This works for any brand.
2. **Brand providers** — one small module per brand that knows that brand's
   dongle protocol (ASUS, Razer, Logitech, …).
3. **Contributors add more providers** — see [CONTRIBUTING.md](CONTRIBUTING.md)
   and `src/providers/template.rs`.

## Supported devices

| Device                                               | Connection                       | Status                      |
|------------------------------------------------------|----------------------------------|-----------------------------|
| ROG Strix Scope II 96 Wireless (keyboard)            | ASUS ROG Omni Receiver           | ✅ tested                   |
| ROG Keris II Ace (mouse)                             | ASUS ROG Omni Receiver           | ✅ tested                   |
| Razer Barracuda X (2022) headset                     | Razer 2.4 GHz dongle `1532:0550` | ✅ tested                   |
| Bluetooth devices with a Windows battery value       | Bluetooth                        | ✅ tested (one headset)     |
| Xbox / PlayStation / other controllers               | Windows                          | ⚠️ untested — please report |
| Razer Barracuda X (2022)                             | Razer dongle `1532:0552`         | ⚠️ untested — please report |
| Other ASUS ROG mice (classic receiver / cable)       | ASUS                             | ⚠️ untested — please report |
| Razer wireless mice (Viper, DeathAdder, Basilisk, …) | Razer dongle                     | ⚠️ untested — please report |
| Logitech Lightspeed / Bolt / Unifying devices        | Logitech receiver                | ⚠️ untested — please report |

**Wanted** (not implemented, we don't guess protocols): SteelSeries, HyperX,
Corsair, Glorious, Pulsar and others. Want yours? See CONTRIBUTING.md.

### Notes and known quirks

- **Razer mice / Logitech:** Charge Deck only opens vendor-defined HID
  collections (see the safety section). If your device only offers its battery
  report on the normal mouse/keyboard collection, Charge Deck cannot read it by design.
- **Barracuda X:** after charging, the headset may stop answering until you
  turn it off and on. Its 2.4 GHz link reports only a voltage, so the % is an
  estimate and "charging" is guessed from the voltage wobble.
- **ASUS:** a sleeping mouse or keyboard cannot be read. The card shows
  "Asleep / off" and the last known value; move the mouse and press Refresh.
- If readings look wrong, close **Armoury Crate / Synapse / G HUB**. They may compete for the dongle.

## How to find your device's VID/PID

Device Manager → your device → Properties → **Details** → Hardware Ids
(`VID_0B05&PID_1ACE` means vendor `0B05`, product `1ACE`).

Or run the probe tool and paste its output into a GitHub issue:

```
charge-deck-probe.exe            list HID interfaces from known brands
charge-deck-probe.exe --all      list every HID interface
charge-deck-probe.exe --verbose  also run all providers, showing every byte (> sent, < received)
```

The listing only reads Windows' device list; it never talks to unknown devices.

## How to build on Windows (beginner friendly)

> 📘 The full step-by-step guide (setup, build, pin, start with Windows, update, uninstall,
> troubleshooting) is in **[INSTALL.md](INSTALL.md)**. The short version:

1. Install Rust from <https://rustup.rs>. It will ask you to install the
   **Visual Studio C++ Build Tools** (Rust needs Microsoft's linker to make `.exe` files).
   Choose "Desktop development with C++". Or run:
   `winget install Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"`
2. Open a new terminal in this folder and run:
   ```
   cargo test
   cargo build --release
   ```
3. Your programs are in `target\release\`: `charge-deck.exe` (the window) and
   `charge-deck-probe.exe` (the debug tool).

### Design preview (no hardware needed)

To see every card state (low, charging, estimated, asleep, checking, disconnected)
or the empty screen, start the app with a fake-device switch. It never scans or
touches any device:

```
set CHARGE_DECK_DEMO=1        & target\release\charge-deck.exe
set CHARGE_DECK_DEMO=2        & target\release\charge-deck.exe   (fewer cards: charging, bars, asleep, disconnected)
set CHARGE_DECK_DEMO=empty    & target\release\charge-deck.exe
```

## Start with Windows

**With the installer:** just tick *Start Charge Deck when I sign in to Windows* on the options page.

**If you built it yourself:**

Press `Win + R`, type `shell:startup`, press Enter, and drop a **shortcut** to
`charge-deck.exe` into that folder.

## Is it safe with anti-cheat?

Charge Deck was designed to stay far away from everything anti-cheat looks for:

- **User mode only.** No kernel driver, no service, no admin rights.
- **Never touches other programs.** No reading or writing other processes' memory, no DLL injection, no process scanning, no hooks into games.
- **No input reading or faking.** No keyboard/mouse hooks, no raw input, no `SendInput`, no macros. It never sees your keypresses or mouse movement.
- **Never opens input collections.** It refuses HID usage page `0x01` (keyboard/mouse/gamepad). It only opens vendor collections (`0xFF00+`) and battery/power collections.
- **Read-only.** It only asks "what's your battery?". It never changes DPI, lighting, key mapping or firmware.
- **Gentle.** One quick check every 5 minutes; it opens a device, asks, and closes it.
- **No overlay.** "Always on top" is a plain window flag. Nothing is drawn inside games.
- **No network, no telemetry, no packing or obfuscation.**

That is how G-Helper, Armoury Crate, Synapse and G HUB read batteries too: they
ask the dongle over USB HID, or ask Windows. But honestly: **no third-party tool
can be guaranteed safe with every game's anti-cheat.** Check your game's rules
and decide for yourself. The source is small and readable so you can verify
the claims above — the rules are enforced in `src/hid_util.rs` and listed in `CONTRIBUTING.md`.

## Credits

See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md). In short:

- Razer Barracuda X protocol: ported from
  [mehdibouchami/razer-barracuda-x-battery](https://github.com/mehdibouchami/razer-barracuda-x-battery) (MIT)
  and [KhromotozzDevOut/razer-barracuda-battery-tray](https://github.com/KhromotozzDevOut/razer-barracuda-battery-tray) (MIT).
- Protocol facts only (no code): G-Helper (GPL-3.0), battery-tray by mikgangal (no license),
  OpenRazer (GPL-2.0), Solaar (GPL-2.0), libratbag (MIT).
- Built with `hidapi`, `windows` and `eframe`/`egui`.

Charge Deck is MIT licensed. It is **not affiliated with ASUS, Razer, Logitech or Microsoft**.
