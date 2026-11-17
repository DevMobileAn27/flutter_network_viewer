; Build only; no automatic publishing or test installation.
#ifndef AppVersion
  #error AppVersion required
#endif
#ifndef OutputName
  #error OutputName required
#endif
[Setup]
AppId=dev.quocan.flutterNetworkViewer.rust
AppName=Flutter Network Viewer
AppVersion={#AppVersion}
DefaultDirName={localappdata}\Programs\Flutter Network Viewer Rust
DefaultGroupName=Flutter Network Viewer
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
CloseApplications=yes
RestartApplications=no
UninstallDisplayIcon={app}\flutter-network-viewer.exe
SetupIconFile=..\assets\icons\AppIcon.ico
OutputDir=..\dist
OutputBaseFilename={#OutputName}
WizardStyle=modern
Compression=lzma2
SolidCompression=yes
[Files]
Source: "..\dist\windows-x64\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs
[Icons]
Name: "{group}\Flutter Network Viewer"; Filename: "{app}\flutter-network-viewer.exe"
