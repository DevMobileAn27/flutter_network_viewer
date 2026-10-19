#ifndef AppVersion
  #error AppVersion must be supplied by the release workflow
#endif
#ifndef OutputName
  #error OutputName must be supplied by the release workflow
#endif

[Setup]
AppId={{38C1BD1F-DFA6-44E8-8594-6F8D575B8040}
AppName=Flutter Network Viewer
AppVersion={#AppVersion}
AppPublisher=DevMobileAn27
AppPublisherURL=https://github.com/DevMobileAn27/flutter_network_viewer
DefaultDirName={localappdata}\Programs\Flutter Network Viewer
DefaultGroupName=Flutter Network Viewer
PrivilegesRequired=lowest
ArchitecturesAllowed=x64compatible
ArchitecturesInstallIn64BitMode=x64compatible
OutputDir=..\build\release
OutputBaseFilename={#OutputName}
SetupIconFile=runner\resources\app_icon.ico
UninstallDisplayIcon={app}\flutter_network_viewer.exe
Compression=lzma2
SolidCompression=yes
WizardStyle=modern
CloseApplications=yes
RestartApplications=no
DisableProgramGroupPage=yes

[Files]
Source: "..\build\windows\x64\runner\Release\*"; DestDir: "{app}"; Flags: ignoreversion recursesubdirs createallsubdirs

[Icons]
Name: "{group}\Flutter Network Viewer"; Filename: "{app}\flutter_network_viewer.exe"

[Run]
Filename: "{app}\flutter_network_viewer.exe"; Flags: nowait runasoriginaluser
