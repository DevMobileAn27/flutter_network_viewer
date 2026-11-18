# Rust build/release — F21

Current source: **1.0.5 (build 15)**. This version restores the original stable
appcast URLs and adds compatibility-feed synchronization on stable publication.
Local commits do not publish a release or repair the public compatibility URLs.

The build workflow creates a GitHub draft after all platform jobs pass.
Publishing remains manual. Release `rust-v1.0.1` shipped without legacy
appcasts. The macOS `rust-v1.0.2` archive has a valid Ed25519 signature, but its
app bundle fails macOS code signature verification; Flutter Network Viewer
`0.1.8` reports a Sparkle validation error. The bundle signing fix was prepared in `1.0.3` and is included in `1.0.4`.

## Identity and update compatibility

Version comes from Cargo and the build number from `packaging/release.json`.
Tags use `rust-v<version>`; bundle/installer ID `dev.quocan.flutterNetworkViewer.rust`.
Windows WinSparkle application name and executable version resource identify Rust.
Settings already use a separate `flutter-network-viewer-rust` directory.

Runtime feeds for new builds:
`https://github.com/DevMobileAn27/flutter_network_viewer/releases/latest/download/appcast-{platform}.xml`.

Compatibility endpoint for installed Rust 1.0.1–1.0.4 clients:
`https://github.com/DevMobileAn27/flutter_network_viewer/releases/download/rust-updates/rust-appcast-{platform}.xml`.
Supported platforms: `macos-arm64`, `macos-x86_64`, `windows-x64`.
Installed Rust versions 1.0.3 and 1.0.4 read this dedicated channel.
The current release pipeline only attached `appcast-*` to versioned releases;
without `rust-updates/rust-appcast-*`, those installed clients receive HTTP 404.
The new `update-channel.yml` workflow copies the three published, signed feeds
to that channel on a stable release publication (or explicit manual dispatch).
It checks version, package identity, feed checksum, signature presence and
prevents version downgrades. The channel is marked prerelease and never latest.
It does not upload installers or change versioned release assets.
The user approved initial channel publication; remote publication and endpoint verification remain pending.

Archives: `flutter-network-viewer-rust-<version>-{platform}.zip`.
Windows updater accepts only the corresponding `-windows-x64-setup.exe`, never
the portable zip. Feed enclosures use `/releases/download/rust-v<version>/…`.
The compatibility synchronization workflow must be present on the default branch
before the next stable publication. New binaries use the original stable URL. The workflow runs on pull requests, pushes to `main`, and manual
dispatch. It uploads versioned workflow artifacts for macOS ARM64, macOS Intel,
and Windows x64. Main runs sign the update packages before upload using the
existing `MACOS_UPDATE_PRIVATE_KEY` and `WINDOWS_UPDATE_PRIVATE_KEY` secrets.
The draft job verifies and attaches `appcast-macos-arm64.xml`,
`appcast-macos-x86_64.xml`, and `appcast-windows-x64.xml`. Flutter `0.1.8`
requests those names from `releases/latest/download`; the draft is only visible
to the updater after it is manually published as latest.

The public keys match Flutter. Signature compatibility and a successful
in-place upgrade are separate: Rust still uses distinct bundle/installer IDs
and settings storage. Validate the migration on an existing `0.1.8`
installation on each OS before relying on it. Signed appcast enclosures also
do not replace OS code signing or notarization.

## Local commands

Use Python >=3.11 for release tools (`/opt/homebrew/bin/python3` on this host).
The system `/usr/bin/python3` is 3.9 and does not contain `tomllib`.

```sh
export CARGO_HOME=/private/tmp/fnv-cargo
export RUSTUP_HOME=/private/tmp/fnv-rustup
export PATH="$CARGO_HOME/bin:$PATH"
cargo fmt --check
cargo clippy --all-targets --locked --offline -- -D warnings
cargo test --locked --offline
/opt/homebrew/bin/python3 -B tools/test_f21_release.py
/opt/homebrew/bin/python3 -B tools/release.py package macos-arm64 --output dist
/opt/homebrew/bin/python3 -B tools/release.py package macos-x86_64 --output dist
/opt/homebrew/bin/python3 -B tools/release.py manifest dist
/opt/homebrew/bin/python3 -B tools/release.py verify dist/SHA256SUMS.txt
```

Install the respective rustup target first. On Windows run the matching
`package windows-x64` command under an MSVC developer environment including
Windows SDK `rc.exe` and `link.exe`. Use developer PowerShell rather than Git
Bash, whose `usr/bin/link.exe` can shadow MSVC. Windows packaging checks the
resolved linker against `VCToolsInstallDir` before Cargo runs. CI passes
`--github-output` to append package metadata to `GITHUB_OUTPUT` after success.
Package staging refuses existing app directories
and archives. It copies the full Sparkle framework preserving symlinks and helpers,
or WinSparkle x64 DLL beside the executable. Assets include platform icons,
Cascadia Mono/OFL, GPL-3.0-only source license and updater notices/licenses.
Dependency license review and corresponding source distribution are still release
gates; the included notices explicitly identify that review as incomplete.

Build Windows setup without installing:

```powershell
& "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe" /DAppVersion=0.1.9 /DOutputName=flutter-network-viewer-rust-0.1.9-windows-x64-setup packaging/windows.iss
python -B tools/release.py manifest dist
```

The installer is per-user/x64, uses a Rust-specific app ID/directory, closes the
running app, installs the updater DLL and assets, and supports WinSparkle's
`/SILENT /NORESTART /SP-` arguments. These are executable release-tool contract
checks on this host; Inno compilation/install/uninstall and executable resource
inspection have **not** run on Windows.

## Signing and feed generation

`package … --identity IDENTITY` provides an inside-out macOS codesign path for
Sparkle XPC services, Updater.app, Autoupdate, framework and application; existing
helper entitlements are preserved, followed by deep/strict verification. This
branch requires a Developer ID identity. Without an identity, the default
macOS package applies an ad-hoc signature to seal the whole app bundle and
preserves the vendor signatures on Sparkle. Both paths require
`codesign --verify --deep --strict` before archiving. Ad-hoc signing is not
Developer ID signing or notarization. Notarization/stapling and Authenticode
signing are not automated here.

`tools/release.py feed PLATFORM ARCHIVE --legacy` reads `UPDATE_SIGNING_KEY`
from the environment and produces `appcast-<platform>.xml`. Omitting `--legacy`
keeps the `rust-appcast-<platform>.xml` filename for the dedicated Rust channel.
Both refuse missing keys and wrong artifact names, and never publish.
The macOS signer accepts the seed over stdin and checks the derived Ed25519
public key against `SUPublicEDKey`. The Windows signer checks the DSA public key,
uses WinSparkle 0.8.x's SHA1 digest signing contract and verifies its output.
Production signing is only exercised when the main workflow can access the
repository secrets. Local signer tests use inert bytes and test keys.

Appcast build/version semantics: Sparkle uses the monotonic build number;
WinSparkle uses the Cargo version. XML, notes, workflow outputs and manifests explicitly use
UTF-8. Signatures and platform/archive identity are required. SHA256 manifests
reject empty, duplicate, malformed, nonlocal and tampered entries. Checksums
validate artifact bytes; native updater signatures authenticate update archives.

## Executed acceptance on 2026-11-13

Evidence is in `docs/artifacts/f21`; earlier feature artifacts were preserved.

| Gate | Actual result |
|---|---|
| Rust core/HTTP + production renderer/input | 270 pass = 213 + 57; zero ignored; `regression-final.log` |
| Rust fmt, all-targets clippy | Pass; `fmt-final.log`, `clippy-final.log` |
| Windows x64 all-targets check + clippy | Pass cross-target; `windows-check-second.log`, `windows-clippy-final.log` |
| Windows linked build | Blocked: `link.exe` unavailable; `windows-build-first.log` |
| Release contracts | 8 Python tests pass; `release-final.log`; metadata/monotonic versions/build, UTF-8, feeds/signature/platform rejection, staging, checksums/tamper, font glyphs/icons, installer/resource/CI contracts |
| Native updater verifier | 2 Python tests pass; `integrity-verified.log`; real Sparkle accepts valid inert archive/rejects unsigned/wrong key/signature/tamper |
| macOS ARM64 + Intel release builds | Pass; `package-final.log`, `intel-final.log`; `lipo` verified both |
| Actual zip contents | Pass; `package-layout-final.json`; SHA256, plist, Sparkle symlink/helper layout, executable architecture, icons/font/licenses |
| Actual packaged main startup | Both pass; `bundle-startup-final/result.json`; Intel via Rosetta on ARM64, not Intel hardware acceptance |
| Fake VM full native flow | Pass; `native-final.log`, `native-final/`; input URL+method filters, pending→201 inspector, clipboard, console, pause/resume, MCP HTTP matches two exported records, Clear/port release |
| DDS→two app sockets/console | Pass; `multiapp-verified.log` and `multiapp-verified/result.json`; injected DDS process listing, real HTTP/probes, choice, first app retained, second scan, shared pause/Clear |
| Live SDK | Pass actual Dart SDK from Flutter install; `live-first.log`, `live-first/live-export.json`; HTTP 201 request/response Unicode, Stdout and developer.log |
| WSS | Self-signed certificate rejection pass via native TLS; `wss-third.log` and server log. Trusted WSS capture pending |
| Native regressions F13–F20 | All pass, source rebuilt; `native-regression.log` and per-feature logs/directories. F17 reload/clipboard, F18 MCP, F19 RAM/log recovery, F20 real Sparkle no-update/available |
| Font and stress | Vietnamese glyph map verified, VI/dark/narrow screenshot visually inspected. 4 renderer cycles warm p95 3.86–3.90ms, cold 112–119ms, RSS baseline 9.7MiB→60.9MiB; `stress-first.json`. Native F16 stress regression also passed |
| Flutter source snapshot | 0 differences; `flutter-source-check.json` |

Additional actual signing-helper verification: `tools/test_f21_signer.py` passes
one macOS test (`signer-verified.log`). An ephemeral test seed signs inert UTF-8
bytes with the production Swift helper, the real Sparkle verifier accepts them,
a wrong trust anchor is rejected and altered bytes fail verification. This does
not exercise the matching production release key or Developer ID signing.

## Remaining release/acceptance gates

- Windows MSVC linked build, PE icon/version resource inspection, Inno setup,
  native GUI/clipboard/persistence/updater/RAM and large-image thumbnail backend.
  Cross-target check/clippy cannot establish these. CI is configured, not run.
- Developer ID signing, notarization/stapling, Authenticode, matching private
  update keys, real release signatures, installer compatibility and complete
  dependency license/source distribution review. No update is installed by tests.
- Trusted WSS full capture, live Flutter application discovery/multi-app GUI,
  accessibility/screen-reader and OS font acceptance on every target. Current
  eframe configuration does not enable its optional AccessKit platform bridge;
  keyboard/input and glyph tests are not screen-reader acceptance.
- Native Intel hardware acceptance; startup under Rosetta is recorded separately.

Retained implementation limits: cold JSON parse/index synchronous; async Logging
object resolution partial; DNS/TCP before socket registration has no absolute
cancel/deadline; MCP owner/UI serialization has no concurrent latency benchmark;
ImageIO limits output to 4096²×4, not total codec memory to 64MiB. Linux RAM backend
remains unverified. F17 owner deadlines, F18 serialized owner HTTP and F19 bounded
metadata-only diagnostics remain unchanged.

Failures and reruns are retained: release tests initially missing module, then
system Python lacking tomllib; initial iconutil iconset rejected (native icns
container generated from existing PNG assets instead); native first run retained
shared search into Console (fixture resets filters, keeps log assertion); discovery
fixture had incomplete header/connection-close behavior (bounded complete-header
peek and request consumption added, assertions retained); OpenSSL system bind and
macOS TLS EOF/alert observation corrected in runner. These are fixture/tool issues,
not claimed production fixes. Windows clippy exposed a needless cfg-only return,
fixed before final cross-target clippy. No coordinator bootstrap failure was
observed in the final full regression; no claim that its earlier cause is fixed.

F21 stays partial. There is no F22 or automatic successor chat.

## Repair installed 1.0.3 clients

Prepared feeds for the published 1.0.4 release are in
`docs/artifacts/update-channel-1.0.4/feeds`. Their signed XML contents are
unchanged; only filenames become `rust-appcast-{platform}.xml`.
No binary rebuild or new version can repair the URL embedded in an already
installed app. The channel must exist at that URL.

Preparation has no remote side effects:

```sh
python -B tools/update_channel.py rust-v1.0.4 --source channel-source --output channel-feeds
```

Only after approval, use `--publish` with GitHub authentication, or manually
dispatch `Sync published Rust update feeds` for `rust-v1.0.4` after the workflow
is merged. Before any writes the tool requires the source release to be public,
stable, and present in the repository. Keep the channel public and retain all
three feed filenames for older installed clients.
