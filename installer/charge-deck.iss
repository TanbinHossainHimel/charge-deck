; Charge Deck installer (Inno Setup 6). Builds the "Next, Next, Finish" setup wizard.
; Build it with installer\build-installer.ps1 (which also builds the app first).
;
; Safety notes (see CONTRIBUTING.md): no admin rights, no driver, no service,
; no network access at install time. It copies the program files, adds
; shortcuts and an uninstaller entry for the CURRENT USER only.

#ifndef AppVersion
  #define AppVersion "0.1.0"
#endif
#define AppName "Charge Deck"
#define AppExe  "charge-deck.exe"

[Setup]
; Never change this GUID: it is how Windows (and the next version of this
; installer) recognises an existing Charge Deck install, so upgrades replace it.
AppId={{6A5CC729-4844-4A81-87F3-B63164685904}
AppName={#AppName}
AppVersion={#AppVersion}
AppVerName={#AppName} {#AppVersion}
AppPublisher=Charge Deck contributors
VersionInfoVersion={#AppVersion}
VersionInfoDescription={#AppName} setup

; Per-user install: no admin prompt. {autopf} becomes %LOCALAPPDATA%\Programs.
PrivilegesRequired=lowest
DefaultDirName={autopf}\{#AppName}
DisableProgramGroupPage=yes
; Fewer clicks: no "choose a folder" page. /DIR="..." on the command line still works.
DisableDirPage=yes
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
MinVersion=10.0

LicenseFile=..\LICENSE
SetupIconFile=..\assets\charge-deck.ico
UninstallDisplayIcon={app}\{#AppExe}
WizardStyle=modern
WizardImageFile=..\assets\wizard-large.bmp
WizardSmallImageFile=..\assets\wizard-small.bmp
; Welcome -> License -> Options -> Install -> Finish.
DisableWelcomePage=no

OutputDir=..\dist
OutputBaseFilename=ChargeDeck-Setup-{#AppVersion}
Compression=lzma2
SolidCompression=yes

; A running Charge Deck would lock its .exe, so the [Code] section below closes it
; just before files are replaced or removed. (The built-in "close applications"
; helper is off: it aborts silent installs when the app doesn't answer it.)
CloseApplications=no
RestartApplications=no

[Languages]
Name: "english"; MessagesFile: "compiler:Default.isl"

[Tasks]
Name: "desktopicon"; Description: "Create a &desktop shortcut"; Flags: unchecked
Name: "startup"; Description: "Start {#AppName} when I sign in to Windows"; Flags: unchecked

[Files]
Source: "..\target\release\charge-deck.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\target\release\charge-deck-probe.exe"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\LICENSE"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\THIRD_PARTY_NOTICES.md"; DestDir: "{app}"; Flags: ignoreversion
Source: "..\README.md"; DestDir: "{app}"; Flags: ignoreversion

[Icons]
Name: "{autoprograms}\{#AppName}"; Filename: "{app}\{#AppExe}"
Name: "{autodesktop}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: desktopicon
; A plain shortcut in the Startup folder (removed again by the uninstaller).
Name: "{userstartup}\{#AppName}"; Filename: "{app}\{#AppExe}"; Tasks: startup

[Run]
Filename: "{app}\{#AppExe}"; Description: "Launch {#AppName}"; Flags: nowait postinstall skipifsilent

[Code]
// Charge Deck keeps no settings or other state, so closing it is harmless.
// Only our own two programs are closed, by exact file name.
procedure StopChargeDeck();
var
  ResultCode: Integer;
begin
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM charge-deck.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Exec(ExpandConstant('{sys}\taskkill.exe'), '/F /IM charge-deck-probe.exe', '', SW_HIDE, ewWaitUntilTerminated, ResultCode);
  Sleep(700);
end;

// Runs after the user clicked Install, right before files are copied.
function PrepareToInstall(var NeedsRestart: Boolean): String;
begin
  StopChargeDeck();
  Result := '';
end;

// Runs when the uninstaller starts.
function InitializeUninstall(): Boolean;
begin
  StopChargeDeck();
  Result := True;
end;
