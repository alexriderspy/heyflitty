# Contributing to Flitty

Thanks for helping. Bug reports, ideas and pull requests are all welcome, especially from Windows users.

## Reporting a bug

[Open an issue](https://github.com/alexriderspy/heyflitty/issues/new/choose) with:

- your Windows (or macOS) version, and x64 or ARM64
- what you asked and what happened
- the message Flitty showed next to the cursor, if any

## Building

You need [Rust](https://rustup.rs), [Node.js](https://nodejs.org) 20+, and on Windows the Visual Studio C++ Build Tools ("Desktop development with C++").

```bash
npm install
npm run tauri dev
```

Windows installer: `npm run tauri build -- --bundles nsis`.

## Testing without an OpenAI key

`scripts/mock-ai/server.py` is a tiny fake OpenAI server. Debug builds read these environment variables:

| Variable | Effect |
|---|---|
| `FLITTY_TEST_OPENAI_BASE_URL` | send requests to this URL instead of OpenAI (e.g. the mock server) |
| `FLITTY_TEST_API_KEY` | use this key instead of the saved one |
| `FLITTY_TEST_TRANSCRIPT` | skip the microphone and ask this question |

Unit tests: `cd src-tauri && cargo test`.

## Pull requests

- Keep changes focused; one fix or feature per PR.
- Run `cargo test` and `npm run build` before opening it.
- If you change behaviour on Windows, say how you tested it.
- If you add a dependency, run `scripts/generate-licenses.sh` and commit the updated `THIRD_PARTY_LICENSES.txt`.

By submitting a pull request you agree to the [CLA](CLA.md).
