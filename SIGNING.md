# Code signing and winget (maintainer guide)

Goal: make the Windows warning *"Windows protected your PC … Unknown publisher"* go away, and let people
install with one command: `winget install AimersLab.ChargeDeck`.

Everything that can be prepared in the repository already is. The steps below are the ones **only the
project owner can do** (they need your accounts or your identity).

> **Both routes need the repository to be public.** SignPath only signs open-source projects, and winget
> downloads the installer from the public Releases page. In GitHub: *Settings → General → Danger Zone →
> Change visibility → Public*.

---

## 1. Code signing with SignPath Foundation (free for open source)

The signed installer shows **"SignPath Foundation"** as the publisher. SignPath Foundation owns the
certificate and is the legal publisher, which is how the free program works.

### 1.1 Apply

1. Make the repository public (see above), and make sure the Releases page has a release.
2. Open <https://signpath.org/apply.html> and fill in the form. Check the current conditions at
   <https://signpath.org/terms> first. Charge Deck fits them as of writing:

| What they ask             | Charge Deck                                                                               |
|---------------------------|-------------------------------------------------------------------------------------------|
| OSI-approved license      | MIT (`LICENSE`)                                                                           |
| Project is public         | `https://github.com/TanbinHossainHimel/charge-deck`                                       |
| Already released          | Releases page, `ChargeDeck-Setup-<version>.exe`                                           |
| Automated build           | `.github/workflows/release.yml` builds from the checked-in source on GitHub's own runners |
| Code signing policy shown | README → *Code signing policy*                                                            |
| Homepage                  | the GitHub repository (a custom domain is **not** required)                               |

3. Wait for the answer. It is a manual review, so it can take days or weeks.

### 1.2 Set up SignPath (after you are approved)

In the SignPath web app:

1. Create the project with the slug **`charge-deck`** and connect it to GitHub (*Trusted Build System*:
   GitHub.com, with origin verification on).
2. Add this **artifact configuration** (the workflow uploads the installer as a zip containing one `.exe`).
   Check the syntax against <https://docs.signpath.io/artifact-configuration> if SignPath complains:

   ```xml
   <artifact-configuration xmlns="http://signpath.io/artifact-configuration/v1">
     <zip-file>
       <pe-file path="ChargeDeck-Setup-*.exe">
         <authenticode-sign/>
       </pe-file>
     </zip-file>
   </artifact-configuration>
   ```

3. Create a signing policy named **`release-signing`**. For open-source projects this normally requires
   *you* to approve each signing request, which is the safety net: nobody can sign something you didn't release.
4. Create an API token for a CI user that is allowed to submit requests to that policy.

### 1.3 Connect it to GitHub

In the repository: *Settings → Secrets and variables → Actions*.

| Type     | Name                           | Value                                    |
|----------|--------------------------------|------------------------------------------|
| Variable | `SIGNPATH_ORGANIZATION_ID`     | your organization id (shown in SignPath) |
| Variable | `SIGNPATH_PROJECT_SLUG`        | `charge-deck`                            |
| Variable | `SIGNPATH_SIGNING_POLICY_SLUG` | `release-signing`                        |
| Secret   | `SIGNPATH_API_TOKEN`           | the token from step 1.2                  |

The release workflow notices these by itself. From the next tag on it builds the installer, sends it to
SignPath, waits for your approval (up to 2 hours), checks that the returned file has a **Valid** signature,
and publishes the signed file with a fresh `.sha256`.

### 1.4 Update the README

Replace the *Code signing policy* section in `README.md` with SignPath's wording, once you are approved:

> Free code signing provided by [SignPath.io](https://about.signpath.io/), certificate by
> [SignPath Foundation](https://signpath.org/).

### 1.5 Check a signed release

Right-click the downloaded installer → **Properties → Digital Signatures**, or in PowerShell:

```powershell
Get-AuthenticodeSignature .\ChargeDeck-Setup-0.2.0.exe | Format-List Status, SignerCertificate
```

`Status` must be `Valid`. SmartScreen may still show a milder "not commonly downloaded" notice for the first
few weeks, because Windows also counts how many people have downloaded a file.

> Only the installer is signed. The `charge-deck.exe` inside it is not. That is enough for the download
> warning. If you want the inner programs signed too, add a second SignPath request before the installer is
> built (ask for help with that when you need it).

---

## 2. winget (`winget install AimersLab.ChargeDeck`)

The package manifest is in `winget/manifests/a/AimersLab/ChargeDeck/0.1.0/`. It passes `winget validate`.
Publisher is **Aimers Lab** (<https://aimers-lab.com>).

> The manifest pins the SHA-256 of the **released 0.1.0 installer**. Don't rebuild or replace that file.
> If you sign a release, publish it as a **new version** (for example 0.1.1) and update winget for that version.

### 2.1 First submission (by hand, once)

1. Make the repository public (the manifest points at the public download URL).
2. Install Microsoft's helper: `winget install Microsoft.WingetCreate`.
3. Submit the prepared manifest. It opens a pull request to Microsoft's
   [winget-pkgs](https://github.com/microsoft/winget-pkgs) repository from *your* GitHub account:

   ```powershell
   wingetcreate submit winget\manifests\a\AimersLab\ChargeDeck\0.1.0
   ```

   (It asks you to sign in to GitHub once.)
4. Microsoft's bots test the package (they install it and scan it). Fix anything the pull request reports.
   A human reviewer then merges it, usually within a few days.
5. When it is merged, `winget install AimersLab.ChargeDeck` works for everyone.

### 2.2 Automatic updates for every new release

1. Create a GitHub **personal access token (classic)** with the `public_repo` scope
   (<https://github.com/settings/tokens>).
2. Save it as the repository secret **`WINGET_TOKEN`**.

From then on, every `v*` tag also runs the `winget` job in the release workflow. It uses `wingetcreate update`
to open the pull request for the new version in winget-pkgs. It does nothing while the secret is missing, or
before the package was accepted the first time.

### 2.3 What users type

```powershell
winget install AimersLab.ChargeDeck
winget upgrade AimersLab.ChargeDeck
winget uninstall AimersLab.ChargeDeck
```

After winget accepts the package, add that command to the README's *Download and install* section.
