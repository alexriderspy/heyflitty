<p align="center">
  <img src="docs/logo.svg" width="96" height="96" alt="Flitty logo" />
</p>

<h1 align="center">Flitty</h1>

<p align="center">
  <b>Your AI screen buddy that shows you exactly where to click.</b><br />
  Hold a hotkey, ask out loud, and Flitty answers in a voice, shows the answer on screen,<br />and flies a little cursor to the exact button you need. Built for Windows, works on macOS.
</p>

<p align="center">
  <a href="https://github.com/alexriderspy/heyflitty/releases/latest"><b>Download for Windows</b></a> ·
  <a href="#macos">macOS</a> ·
  <a href="https://github.com/alexriderspy/heyflitty/issues/new/choose">Report an issue</a> ·
  <a href="CONTRIBUTING.md">Contribute</a>
</p>

---

## What it does

1. **Hold `Ctrl` + `Alt` + `Space`** anywhere and ask a question out loud, like "where do I turn on dark mode?"
2. **Let go.** Flitty looks at your screen and answers in a voice.
3. **It points.** A small cursor flies across your screen to the exact button or menu it's talking about.

It works over any app, including full-screen ones, and never gets in the way: clicks and typing pass straight through. Everything it says is also captioned next to the cursor.

## Why Flitty

Flitty is inspired by HeyClicky, which showed how good an AI buddy on your screen can feel, but it's Mac-only and closed. Flitty brings the idea to Windows:

- **Windows first.** Windows 10 and 11, x64 and ARM64. macOS works too.
- **Free and open source.** MIT licensed. No account, no subscription.
- **Your key, your data.** Uses your own OpenAI API key, talking to OpenAI directly. There's no Flitty server.

## Requirements

- Windows 10 or 11 (x64 or ARM64), or macOS 14.2+ (see [macOS](#macos)).
- An [OpenAI API key](https://platform.openai.com/api-keys). You pay OpenAI directly for what you use.
- A microphone.

## Install

1. Download `Flitty_<version>_x64-setup.exe` from the [latest release](https://github.com/alexriderspy/heyflitty/releases/latest) (`arm64` for Snapdragon laptops).
2. Run it. It installs for your user account only, no admin rights needed.
3. Click the Flitty icon in the system tray, open **Settings**, paste your OpenAI key and press **Test key**.

The installer isn't code-signed yet, so Windows may show "Windows protected your PC". Click **More info → Run anyway**.

### macOS

Flitty runs on macOS too. There's no signed Mac download yet, so for now run it from source (see [Build from source](#build-from-source)). The first time, macOS asks for Microphone and Screen Recording permission.

## How it works

- While you hold the hotkey, Flitty records your microphone.
- When you let go, it sends the recording to OpenAI for transcription, takes a screenshot of your displays, and sends your question and the screenshot to an OpenAI model.
- The reply streams back and is spoken sentence by sentence with your system's built-in voice.
- If the answer is about something on screen, the model includes its coordinates and Flitty's cursor flies there.

## Privacy

- Flitty only records while you **hold the hotkey**, and takes a screenshot only when you **let go** of it.
- Audio and screenshots go **straight from your computer to OpenAI** using your own key. There is no Flitty server.
- Recordings and screenshots are never saved to disk. Your API key is kept in Windows Credential Manager (Keychain on macOS).

## Build from source

You need [Rust](https://rustup.rs), [Node.js](https://nodejs.org) 20+, and on Windows the Visual Studio C++ Build Tools.

```bash
git clone https://github.com/alexriderspy/heyflitty.git
cd heyflitty
npm install
npm run tauri dev
```

Build the Windows installer with `npm run tauri build -- --bundles nsis`.

## Report issues and contribute

- **Found a bug?** [Open an issue](https://github.com/alexriderspy/heyflitty/issues/new/choose). Include your Windows version and what Flitty said next to the cursor.
- **Have an idea?** Open a feature request, or just send a pull request. See [CONTRIBUTING.md](CONTRIBUTING.md) for how to build and test.

Contributions require agreeing to the [CLA](CLA.md).

## License

MIT. See [LICENSE](LICENSE).

Flitty's system prompt is adapted from [Clicky](https://github.com/farzaa/clicky) (MIT); see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md). Licenses of all bundled dependencies are in [THIRD_PARTY_LICENSES.txt](THIRD_PARTY_LICENSES.txt), regenerated with `scripts/generate-licenses.sh` and shipped with the installer.
