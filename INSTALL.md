# Installing Charge Deck

Charge Deck installs like any normal Windows program: **download one file, double-click it, click
Next a few times.** No admin rights, no driver and no service are needed.

**You need:** Windows 10 or 11 (64-bit).

---

## Option A — Download the installer (recommended)

### 1. Download

Go to the **Releases** page of the Charge Deck GitHub project and download
**`ChargeDeck-Setup-<version>.exe`** (about 5 MB) from the newest release.

*Optional check:* next to it there is a `.sha256` file. To make sure the download is intact, run this in
PowerShell and compare it with the number in that file:

```powershell
Get-FileHash .\ChargeDeck-Setup-0.1.0.exe -Algorithm SHA256
```

### 2. Run it

Double-click the file you downloaded.

> **"Windows protected your PC" (SmartScreen)?** Windows shows this for any new program that isn't
> digitally signed yet, even harmless ones. Click **More info**, then **Run anyway**.
> This is normal for Charge Deck today. See [RELEASING.md](RELEASING.md) for why, and how it gets better.

### 3. Click through the wizard

| Step | What you see                                | What to do                                                                                                    |
|------|---------------------------------------------|---------------------------------------------------------------------------------------------------------------|
| 1    | **Welcome to the Charge Deck Setup Wizard** | Click **Next**                                                                                                |
| 2    | **License Agreement** (MIT license)         | Choose *I accept the agreement*, click **Next**                                                               |
| 3    | **Select Additional Tasks**                 | Optional boxes: *Create a desktop shortcut* and *Start Charge Deck when I sign in to Windows*. Click **Next** |
| 4    | **Ready to Install**                        | Click **Install** (takes a few seconds)                                                                       |
| 5    | **Completing the Charge Deck Setup Wizard** | Leave *Launch Charge Deck* ticked, click **Finish**                                                           |

Charge Deck opens, and one card appears for each device it finds after about 10 seconds.
You can also start it any time from the **Start menu → Charge Deck**.

Where it goes: `C:\Users\<you>\AppData\Local\Programs\Charge Deck` (only for your Windows account,
which is why no administrator password is asked). The wizard doesn't ask for a folder; advanced users can
choose one with `/DIR="D:\Apps\ChargeDeck"` (see *Quiet install* below).

### Updating

Download the newer `ChargeDeck-Setup-…exe` and run it. It replaces the old version and keeps your
shortcuts. If Charge Deck is open, the installer closes it for you and installs over it (Charge Deck
keeps no settings, so nothing is lost). Uninstalling does the same.

### Uninstalling

**Settings → Apps → Installed apps → Charge Deck → Uninstall.** This removes the program, its
shortcuts and the start-with-Windows entry. Charge Deck keeps no settings or other files, so nothing is
left behind.

### Quiet install (for advanced users and IT)

```powershell
.\ChargeDeck-Setup-0.1.0.exe /VERYSILENT /SUPPRESSMSGBOXES /TASKS="desktopicon,startup"
```

Leave out `/TASKS=…` for no extra shortcuts, or pass `/DIR="D:\Apps\ChargeDeck"` to choose the folder.

---

## Check that your devices are found

1. Switch on your wireless devices and plug in their USB receivers or dongles.
2. Open Charge Deck and wait about 10 seconds. Press **Refresh** to scan again. It also refreshes
   by itself every 5 minutes.

A device shows **Asleep / off** when its receiver answers but the device itself doesn't, for example a
mouse that went to sleep. Move it and press **Refresh**.

If a device is missing, run the diagnostic tool that was installed next to the app. In PowerShell:

```powershell
& "$env:LOCALAPPDATA\Programs\Charge Deck\charge-deck-probe.exe" --verbose
```

It lists the USB devices it knows about, whether Windows has a Bluetooth battery value, and every byte it
sends and receives. Copy that output into a GitHub issue. The probe only *lists* unknown devices and
never talks to them.

---

## Troubleshooting

| Problem                                      | Fix                                                                                                                                                                                                                                      |
|----------------------------------------------|------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| Windows says "protected your PC"             | Click **More info → Run anyway**. The installer isn't signed yet (see above).                                                                                                                                                            |
| Antivirus blocks or deletes the installer    | New unsigned programs sometimes trigger this. Check the SHA-256 (step 1), then allow the file. Charge Deck never touches other programs, never reads input and never uses the network (see *Is it safe with anti-cheat?* in the README). |
| The window doesn't open, or is blank         | Update your graphics driver. The app draws with DirectX 12 or Vulkan, which almost every PC from the last decade supports.                                                                                                               |
| A device shows wrong or missing values       | Close **Armoury Crate / Razer Synapse / Logitech G HUB**, they can compete for the dongle. Then press **Refresh**.                                                                                                                       |
| Razer headset stops answering after charging | Switch the headset off and on again.                                                                                                                                                                                                     |
| Razer or Logitech mouse not found            | Those providers are untested and only use vendor HID collections. Run the probe and open a GitHub issue.                                                                                                                                 |

---

## Option B — Build it yourself from the source code

For developers, or if you prefer to build the program rather than trust a download.

### 1. One-time setup

**C++ Build Tools** (Rust needs Microsoft's linker). In PowerShell **as Administrator**:

```powershell
winget install Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

(Or download *Build Tools for Visual Studio* and tick **Desktop development with C++**. Visual Studio
Code alone is not enough.)

**Rust**: download `rustup-init.exe` from <https://rustup.rs>, run it, press Enter, then **open a new
terminal**. Check with `cargo --version`. You need **Rust 1.85 or newer** (`rustup update` if older).

### 2. Build

```powershell
git clone <the Charge Deck repository URL>
cd charge-deck
cargo build --release
```

Your programs are now in `target\release\`: `charge-deck.exe` (the app, about 11 MB) and
`charge-deck-probe.exe`. Both are self-contained, so you can run them from anywhere.

### 3. Optional: build the same setup wizard yourself

```powershell
winget install JRSoftware.InnoSetup
powershell -File installer\build-installer.ps1
```

The wizard is written to `dist\ChargeDeck-Setup-<version>.exe`.

### Build troubleshooting

| Problem                                | Fix                                                                                                                                |
|----------------------------------------|------------------------------------------------------------------------------------------------------------------------------------|
| `linker 'link.exe' not found`          | The C++ Build Tools are missing or half-installed. Redo step 1, then open a **new** terminal.                                      |
| `cargo` is not recognized              | Close the terminal and open a new one. In **Git Bash**, add Rust once: `echo 'export PATH="$PATH:$HOME/.cargo/bin"' >> ~/.bashrc`. |
| Build says the edition isn't supported | Your Rust is too old. Run `rustup update`.                                                                                         |

---

## Want to see the design without any devices?

Charge Deck has a preview mode with fake devices. It never scans or touches hardware:

```powershell
$env:CHARGE_DECK_DEMO = "1"; & "$env:LOCALAPPDATA\Programs\Charge Deck\charge-deck.exe"
```

Use `"2"` for fewer cards, or `"empty"` for the empty screen. Run `Remove-Item Env:\CHARGE_DECK_DEMO`
afterwards to go back to normal.
