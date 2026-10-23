# Publishing with winget (and code signing later)

Goal: let people install Charge Deck with one command: `winget install AimersLab.ChargeDeck`.

**Current plan: winget only.** Code signing with SignPath is prepared but parked, see
[Later: code signing](#later-code-signing-with-signpath-parked) at the end.

> **winget needs the repository to be public.** winget downloads the installer from the public Releases page.
> In GitHub: *Settings → General → Danger Zone → Change visibility → Public*.

---

## 1. Publish on winget

The package manifest is in `winget/manifests/a/AimersLab/ChargeDeck/0.1.0/`. It passes `winget validate`.
Publisher is **Aimers Lab** (<https://aimers-lab.com>).

> The manifest pins the SHA-256 of the **released 0.1.0 installer**. Don't rebuild or replace that file.
> New versions get a new manifest (step 2).

### 1.1 First submission (by hand, once)

1. Make the repository public (see above), and check that the installer downloads without being logged in.
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

### 1.2 What users type

```powershell
winget install AimersLab.ChargeDeck
winget upgrade AimersLab.ChargeDeck
winget uninstall AimersLab.ChargeDeck
```

After winget accepts the package, add that command to the README's *Download and install* section and to
`INSTALL.md`.

---

## 2. Automatic winget updates for every new release

Do this after the first submission was merged.

1. Create a GitHub **personal access token (classic)** with the `public_repo` scope
   (<https://github.com/settings/tokens>).
2. Save it as the repository secret **`WINGET_TOKEN`**.

From then on, every `v*` tag also runs the `winget` job in `.github/workflows/release.yml`. It uses
`wingetcreate update` to open the pull request for the new version in winget-pkgs. It does nothing while
the secret is missing, or before the package was accepted the first time.

---

## About the "Windows protected your PC" warning

Until the installer is code-signed, Windows SmartScreen shows "Unknown publisher" when a user runs a file
**downloaded in a browser**. `INSTALL.md` tells people to click *More info → Run anyway*, and every release has a
SHA-256 checksum to check the download.

Installing through winget is a separate path. winget checks the SHA-256 in the manifest itself, and
Microsoft's bots scan every package before it is accepted. I haven't confirmed whether winget installs of an
**unsigned** installer show any warning, so watch for it when you test your own package.

---

## Later: code signing with SignPath (parked)

Not needed for winget. Nothing here is active: the release workflow only signs if you add the settings below,
and it builds unsigned otherwise.

The signed installer would show **"SignPath Foundation"** as the publisher (SignPath Foundation owns the
free certificate). Check the current conditions at <https://signpath.org/terms> before applying.

### Apply

1. The repository must be public and have a release.
2. Apply at <https://signpath.org/apply.html>. Charge Deck fits as of writing:

| What they ask             | Charge Deck                                                                               |
|---------------------------|-------------------------------------------------------------------------------------------|
| OSI-approved license      | MIT (`LICENSE`)                                                                           |
| Project is public         | `https://github.com/TanbinHossainHimel/charge-deck`                                       |
| Already released          | Releases page, `ChargeDeck-Setup-<version>.exe`                                           |
| Automated build           | `.github/workflows/release.yml` builds from the checked-in source on GitHub's own runners |
| Code signing policy shown | README → *Code signing policy* (update it with SignPath's wording after approval)         |
| Homepage                  | the GitHub repository (a custom domain is **not** required)                               |

### Set up SignPath (after approval)

1. Create the project with the slug **`charge-deck`** and connect it to GitHub (*Trusted Build System*:
   GitHub.com, with origin verification on).
2. Add this **artifact configuration**. Check the syntax against
   <https://docs.signpath.io/artifact-configuration> if SignPath complains:

   ```xml
   <artifact-configuration xmlns="http://signpath.io/artifact-configuration/v1">
     <zip-file>
       <pe-file path="ChargeDeck-Setup-*.exe">
         <authenticode-sign/>
       </pe-file>
     </zip-file>
   </artifact-configuration>
   ```

3. Create a signing policy named **`release-signing`** (open-source projects normally require *you* to approve
   each request) and an API token for a CI user that may submit to it.
4. In GitHub: *Settings → Secrets and variables → Actions*:

| Type     | Name                           | Value                                    |
|----------|--------------------------------|------------------------------------------|
| Variable | `SIGNPATH_ORGANIZATION_ID`     | your organization id (shown in SignPath) |
| Variable | `SIGNPATH_PROJECT_SLUG`        | `charge-deck`                            |
| Variable | `SIGNPATH_SIGNING_POLICY_SLUG` | `release-signing`                        |
| Secret   | `SIGNPATH_API_TOKEN`           | the token from step 3                    |

From the next tag on, the workflow signs the installer, checks that the signature is **Valid**, and
publishes the signed file with a fresh `.sha256`. A signed release must be a **new version**, because the
winget manifest and users' checksums are tied to the exact file of each version.
