# Contributing: how to add support for your device

Thank you! Adding a brand = **one new file + one line** in the registry.

## Step by step

1. **Find your IDs.** Run `charge-deck-probe.exe --all` and look for your
   device: VID:PID, usage page, usage. (Or Device Manager → Details → Hardware Ids.)
2. **Copy the template.** `src/providers/template.rs` → `src/providers/yourbrand.rs`.
3. **Match your devices.** In `is_ours`, match **vendor id + product id + usage page**. Nothing broader.
4. **Implement `scan`.** Open the collection with `open_if_allowed`, `flush`,
   send the battery query with `write_report`, wait with `read_until`.
5. **Keep parsing pure.** Write `fn parse_answer(bytes: &[u8]) -> ...` and unit test it
   with **real captured bytes** (no hardware needed to run the tests).
6. **Register it.** In `src/providers/mod.rs` add `pub mod yourbrand;` and one line in `registry()`:
   `Box::new(yourbrand::YourBrand::default()),`
   `Provider::name()` must be **unique** (there is a test for it).
7. **Run** `cargo test` and `cargo build --release`.
8. **Update the README table.** Mark it ✅ tested only if you tested it on real hardware, else ⚠️ untested.
9. **Open a pull request** with a probe `--verbose` output from your device.

## SAFETY RULES — mandatory for every provider

Pull requests that break any of these are **rejected**.

1. **User-mode only.** No kernel drivers, no services, no installers. Never require admin rights.
2. **Never touch other processes.** No `OpenProcess`, no reading/writing other programs' memory, no DLL injection, no process scanning, no hooks into games.
3. **No input reading or faking.** No `SetWindowsHookEx`, no raw input, no `SendInput`, no macros. Never read keypresses or mouse movement (for controllers: battery APIs only, never button/stick state).
4. **Never open input collections.** Never open HID usage page `0x01` (keyboard, mouse, joystick, gamepad). Only vendor collections (`0xFF00` and up) and battery/power pages (`0x06`, `0x84`, `0x85`). Always open through `hid_util::open_if_allowed`.
5. **Read-only commands only.** Only battery/status queries. Never change DPI, polling rate, lighting, key mapping, macros, firmware or any setting. **Put a comment above every `write()` / feature-report send saying it is read-only.**
6. **Never guess.** Only send vendor commands to devices your provider explicitly knows (VID + PID/usage page). Never "try commands" on unknown devices.
7. **No game overlay.** No DirectX/Vulkan/OpenGL hooking, no drawing inside games.
8. **Gentle polling.** Open the device, ask, close it. Don't hold devices open between checks. Short timeouts (a few seconds at most per provider).
9. **No network access.** No telemetry, no update checks.
10. **No packing or obfuscation.** No UPX, no encrypted strings. Plain readable code.
11. **No suspicious names** in code, files or the exe (nothing like "inject", "hook", "bypass", "cheat").
12. **Honesty in the README.** Don't claim anti-cheat safety beyond what the README says.
13. **One provider failing must never stop the others.** Never panic; return an empty list instead. (The runner also catches panics, but don't rely on it.)

## License rule

Charge Deck is **MIT**. Some great projects are **GPL** (OpenRazer, Solaar,
HeadsetControl, G-Helper). **Do not copy or port code from GPL projects.**
Learning protocol facts (which byte means what) is fine; write your own code.
Code ported from MIT projects is fine, but add their copyright notice to
`THIRD_PARTY_NOTICES.md`. **Check the license of every project before porting** and
mention it in your PR. A project with no license file is "all rights reserved": facts only.

## How to capture bytes safely

- **The probe:** `charge-deck-probe.exe --verbose` prints every byte sent (`>`) and received (`<`) by existing providers.
- **USBPcap + Wireshark:** start a capture, open the *official* app so it reads the
  battery, stop the capture, and look for the request/answer pair. Only capture
  **battery reads**. Do not click settings in the official app while capturing
  (and never replay captures that change settings).
- Paste the relevant bytes into your unit test, with a comment saying what they mean.
