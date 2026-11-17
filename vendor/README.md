Updater runtime provenance (F20)

- Sparkle 2.9.6 / CFBundleVersion 2061, universal ARM64+x86_64 framework and
  sign_update verifier copied from the existing Flutter project's CocoaPods
  Sparkle installation. LICENSE retained. https://sparkle-project.org/
- WinSparkle 0.8.1 x64 DLL and API header copied from the installed
  auto_updater_windows 1.0.0 Flutter plugin's WinSparkle-0.8.1 distribution.
  COPYING, COPYING.expat and other bundled COPYING files retained.
  https://winsparkle.org/
- SHA256SUMS pins regular runtime/header/license files. Verify before building
  or bundling with python3 -B tools/verify_updater_vendor.py. This manifest is
  source-controlled input; changing it must be reviewed along with the runtime.
- No private release keys. native/windows_dsa_pub.pem is the existing public
  verification key; macOS SUPublicEDKey is a bundle packaging requirement.
- Native installation is never exercised by tests. Packaging, code signing,
  installer/feed generation and publication are outside F20.
