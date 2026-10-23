# Releasing Charge Deck (for maintainers)

How a new version gets from the source code to a download that users click through.

## 1. Build the installer locally

```powershell
winget install JRSoftware.InnoSetup      # once
powershell -File installer\build-installer.ps1
```

This builds the app in release mode and packs it into `dist\ChargeDeck-Setup-<version>.exe`, plus
`.sha256` with its checksum. The version comes from `Cargo.toml` (one place only).

Before you publish, test it on a clean user account or machine: run the wizard, start the app,
uninstall, and check that the Start menu entry and the *Installed apps* entry are gone.

The installer follows the same rules as the app (see CONTRIBUTING.md): **per-user install, no admin
rights, no driver, no service, no network access.** Don't add any of those to `installer\charge-deck.iss`.
Never change the `AppId` GUID in that file: it is how a new version finds and replaces an old one.

## 2. Publish on GitHub (automatic)

The workflow `.github/workflows/release.yml` does it for you when you push a version tag:

1. Change `version` in `Cargo.toml` (for example to `0.2.0`) and commit.
2. `git tag v0.2.0` then `git push origin v0.2.0`.
3. GitHub builds the installer on a Windows machine and attaches `ChargeDeck-Setup-0.2.0.exe` and its
   `.sha256` to a new release. Users download it from the **Releases** page.

The tag must match the `Cargo.toml` version, or the workflow stops with an error.
You can also start the workflow by hand from the **Actions** tab to get an installer artifact without
publishing a release.

## 3. The SmartScreen warning (read this)

Windows SmartScreen shows **"Windows protected your PC"** for programs that are not signed with a
code-signing certificate, and for signed programs until they have built up a download reputation.
It is not a virus detection. It is just "we don't know this publisher yet". Right now Charge Deck
is **unsigned**, so every user sees it once and has to click **More info → Run anyway**. INSTALL.md
tells them so.

The release workflow is already prepared for **SignPath Foundation** signing and for **winget**.
The steps that only you can do (applying, creating tokens) are in **[SIGNING.md](SIGNING.md)**.

Other ways to improve this (each involves an application or a cost, so it is your decision):

| Option                                | Notes                                                                                                 |
|---------------------------------------|-------------------------------------------------------------------------------------------------------|
| **SignPath Foundation**               | Free code signing for open-source projects. You apply, and signing runs from the GitHub workflow.     |
| **Azure Trusted Signing**             | Microsoft's low-cost signing service, billed monthly.                                                 |
| **A normal code-signing certificate** | Paid yearly. An *EV* certificate gives instant SmartScreen trust; others need reputation to build up. |

Signing the installer (and the `.exe` inside it) removes the "unknown publisher" wording at once.
SmartScreen's "not commonly downloaded" notice can still show for the first weeks.

Other things that help, all already done: the `.exe` is plain (no packers, no obfuscation), carries a
proper name, icon and version, and the installer is built with the standard Inno Setup tool.

## 4. After a release

- Add the release link to the README's download section.
- If users report antivirus false positives, submit the installer to the vendor's false-positive form
  (Microsoft Defender: <https://www.microsoft.com/wdsi/filesubmission>).
