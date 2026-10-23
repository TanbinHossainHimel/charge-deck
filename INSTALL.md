# Installing Charge Deck

Charge Deck is a single `.exe` file. There is **no installer, no driver, no service and no admin step**
for the app itself. You just build it once (or copy a built `.exe`), run it, and optionally make it
start with Windows.

> **Right now there are no pre-built downloads.** You build it yourself from the source code. It takes
> about 15 minutes the first time (most of it is downloading tools), then about 2 minutes for every
> build after that. Step by step below.

**You need:** Windows 10 or 11 (64-bit), an internet connection for the one-time setup, and several
GB of free disk space for the build tools.

---

## Part 1 — One-time setup (about 10 minutes)

You need two free tools: the **C++ Build Tools** (Rust needs Microsoft's linker to create `.exe` files)
and **Rust** itself.

### 1.1 Install the C++ Build Tools

Open **PowerShell as Administrator** (right-click the Start button → *Terminal (Admin)*) and run:

```powershell
winget install Microsoft.VisualStudio.2022.BuildTools --override "--wait --passive --add Microsoft.VisualStudio.Workload.VCTools --includeRecommended"
```

A small progress window appears and asks nothing. Wait until it finishes (several minutes).

*No `winget`?* Download **Build Tools for Visual Studio** from
<https://visualstudio.microsoft.com/downloads/>, run it, and tick **Desktop development with C++**.

> Visual Studio Code is **not** enough. You need the Build Tools above.

### 1.2 Install Rust

1. Go to <https://rustup.rs> and download **rustup-init.exe**.
2. Run it and press **Enter** to accept the default installation.
3. When it finishes, **close the terminal and open a new one** so Windows picks up the new `PATH`.

Check that it worked:

```powershell
cargo --version
```

You should see something like `cargo 1.85.0` or newer. Charge Deck needs **Rust 1.85 or newer**
(it uses the 2024 edition). If yours is older, run `rustup update`.

### 1.3 Get the Charge Deck source code

If you have Git:

```powershell
git clone <the Charge Deck repository URL>
cd charge-deck
```

If you don't, download the repository as a ZIP from GitHub, unzip it, and open a terminal inside the
unzipped `charge-deck` folder.

---

## Part 2 — Build it

In the `charge-deck` folder run:

```powershell
cargo build --release
```

The first build downloads and compiles the libraries, so it takes a few minutes. When you see
`Finished release profile`, your programs are here:

| File                                   | What it is                                                     |
|----------------------------------------|----------------------------------------------------------------|
| `target\release\charge-deck.exe`       | **The app** (the window with all your batteries). About 11 MB. |
| `target\release\charge-deck-probe.exe` | A small console tool for checking which devices are found.     |

Optional: run the tests (`cargo test`). They don't need any hardware.

---

## Part 3 — Put it somewhere permanent

The `.exe` is fully self-contained (even the font is inside it), so you can copy it anywhere.

1. Make a folder, for example `C:\Tools\ChargeDeck`.
2. Copy `charge-deck.exe` and `charge-deck-probe.exe` into it.
3. Double-click `charge-deck.exe`. A window called **Charge Deck** opens and starts looking for your devices.

**Pin it:** right-click `charge-deck.exe` → *Show more options* → *Create shortcut*, then drag the shortcut to
your desktop. Or right-click the shortcut → *Pin to Start* / *Pin to taskbar*.

### First run: Windows SmartScreen warning

Because the `.exe` is not code-signed, Windows may show a blue **"Windows protected your PC"** window.
That is expected for a program you built yourself: click **More info → Run anyway**.
Since you built it from the source code, you know exactly what is inside.

---

## Part 4 — Start with Windows (optional)

1. Press **Win + R**, type `shell:startup`, press **Enter**. A folder opens.
2. Drag a **shortcut** to `charge-deck.exe` into that folder (hold **Alt** while dragging, or use
   *Create shortcut* as above).

Next time you sign in, Charge Deck opens by itself. To stop that, delete the shortcut from the same folder.

---

## Part 5 — Check that your devices are found

1. Switch on your wireless devices and plug in their USB receivers or dongles.
2. Open `charge-deck.exe` and wait about 10 seconds. One card should appear per device.
   Press **Refresh** to scan again. It also refreshes by itself every 5 minutes.

A device shows **Asleep / off** when its receiver answers but the device itself doesn't, for example a
mouse that went to sleep. Move it and press **Refresh**.

If a device is missing, open a terminal in the folder that holds the `.exe` files and run:

```powershell
.\charge-deck-probe.exe --verbose
```

It prints every USB device it knows about, whether Windows has a Bluetooth battery value, and every byte
it sends and receives. Copy that output into a GitHub issue. The probe only *lists* unknown devices
and never talks to them.

---

## Updating

```powershell
git pull
cargo build --release
```

Then copy the new `charge-deck.exe` over the old one (close the app first).

## Uninstalling

Close Charge Deck, then delete the folder with the `.exe` files and any shortcuts (including the one in
`shell:startup`). That's all: Charge Deck installs nothing else, writes no settings and adds nothing to
the registry.

---

## Troubleshooting

| Problem                                             | Fix                                                                                                                                                                                                                                                                                                             |
|-----------------------------------------------------|-----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------|
| `linker 'link.exe' not found` when building         | The C++ Build Tools are missing or half-installed. Redo step 1.1, then open a **new** terminal.                                                                                                                                                                                                                 |
| `cargo` is not recognized                           | Close the terminal and open a new one. In **Git Bash**, add Rust once: `echo 'export PATH="$PATH:$HOME/.cargo/bin"' >> ~/.bashrc`, then open a new Git Bash window. PowerShell usually works without this.                                                                                                      |
| Build says the edition or a feature isn't supported | Your Rust is too old. Run `rustup update`.                                                                                                                                                                                                                                                                      |
| The window doesn't open, or is blank                | Update your graphics driver. The app draws with DirectX 12 or Vulkan, which almost every PC from the last decade supports.                                                                                                                                                                                      |
| A device shows wrong or missing values              | Close **Armoury Crate / Razer Synapse / Logitech G HUB**. They can compete with Charge Deck for the dongle. Then press **Refresh**.                                                                                                                                                                             |
| Antivirus flags the `.exe`                          | The `.exe` is unsigned and new, so some antivirus programs are suspicious of it. Build it yourself from the source (as above) and add an exclusion for that folder. Charge Deck never touches other programs, never reads input, and never uses the network. See *"Is it safe with anti-cheat?"* in the README. |
| Razer headset stops answering after charging        | Switch the headset off and on again.                                                                                                                                                                                                                                                                            |
| Razer or Logitech mouse not found                   | Those providers are untested and only use vendor HID collections. Run the probe and open a GitHub issue.                                                                                                                                                                                                        |

---

## Want to see the design without any devices?

Charge Deck has a preview mode with fake devices. It never scans or touches hardware:

```powershell
$env:CHARGE_DECK_DEMO = "1"; .\charge-deck.exe
```

Use `"2"` for fewer cards, or `"empty"` for the empty screen. Close that terminal (or run
`Remove-Item Env:\CHARGE_DECK_DEMO`) to go back to normal.
