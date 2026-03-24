# CloudpilotEmu Tauri Shim

Tauri v2 wrapper for the [CloudpilotEmu PWA](https://cloudpilot-emu.github.io/app).

## Prerequisites

- [Node.js](https://nodejs.org/) (v18+)
- [Rust](https://rustup.rs/)
- Platform-specific dependencies: see [Tauri prerequisites](https://v2.tauri.app/start/prerequisites/)

## Setup

```sh
npm install
```

## Build

### macOS

```sh
npx tauri build
```

Outputs `.app` and `.dmg` in `src-tauri/target/release/bundle/`.

### Windows

```sh
npx tauri build
```

Outputs NSIS installer and MSI in `src-tauri/target/release/bundle/`.

### Linux

```sh
npx tauri build
```

Outputs AppImage and `.deb` in `src-tauri/target/release/bundle/`.

### Android

Requires [Android SDK and NDK](https://v2.tauri.app/start/prerequisites/#android).

```sh
npx tauri android init   # first time only
npx tauri icons          # generate icons in android project
npx tauri android build
apksigner sign --ks ~/.android/debug.keystore --ks-pass pass:android  ./src-tauri/gen/android/app/build/outputs/apk/universal/release/app-universal-release-unsigned.apk # codesign with debug cert
```

Outputs APK/AAB in `src-tauri/gen/android/app/build/outputs/`.

## Development

```sh
npx tauri dev            # desktop
npx tauri android dev    # android
```

`devUrl` is set to `http://localhost:4200` — start the PWA dev server separately if needed.
