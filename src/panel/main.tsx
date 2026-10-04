import { StrictMode, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import "./panel.css";

type ChatApi = "anthropic" | "open-ai-compatible";
type Settings = {
  chat: { api: ChatApi; baseUrl: string; model: string };
  transcription: { baseUrl: string; model: string };
  voice: { voiceName: string; muted: boolean };
};
type SettingsView = { settings: Settings; hasChatKey: boolean; hasTranscriptionKey: boolean };

const CHAT_PRESETS: { name: string; api: ChatApi; baseUrl: string; model: string }[] = [
  { name: "Anthropic", api: "anthropic", baseUrl: "https://api.anthropic.com", model: "claude-sonnet-5-5" },
  { name: "OpenAI", api: "open-ai-compatible", baseUrl: "https://api.openai.com/v1", model: "gpt-5" },
  { name: "Google Gemini", api: "open-ai-compatible", baseUrl: "https://generativelanguage.googleapis.com/v1beta/openai", model: "gemini-2.5-flash" },
  { name: "OpenRouter", api: "open-ai-compatible", baseUrl: "https://openrouter.ai/api/v1", model: "openrouter/auto" },
  { name: "Ollama (local)", api: "open-ai-compatible", baseUrl: "http://localhost:11434/v1", model: "qwen2.5vl" },
];

const TRANSCRIPTION_PRESETS = [
  { name: "OpenAI", baseUrl: "https://api.openai.com/v1", model: "gpt-4o-mini-transcribe" },
  { name: "Groq", baseUrl: "https://api.groq.com/openai/v1", model: "whisper-large-v3-turbo" },
];

function presetName(presets: { name: string; baseUrl: string }[], baseUrl: string) {
  return presets.find((preset) => preset.baseUrl === baseUrl)?.name ?? "Custom";
}

function useSystemVoices() {
  const [voices, setVoices] = useState<SpeechSynthesisVoice[]>(() => speechSynthesis.getVoices());
  useEffect(() => {
    const update = () => setVoices(speechSynthesis.getVoices());
    speechSynthesis.addEventListener("voiceschanged", update);
    return () => speechSynthesis.removeEventListener("voiceschanged", update);
  }, []);
  return voices;
}

function Panel() {
  const [view, setView] = useState<SettingsView | null>(null);
  const [draft, setDraft] = useState<Settings | null>(null);
  const [chatKey, setChatKey] = useState("");
  const [transcriptionKey, setTranscriptionKey] = useState("");
  const [status, setStatus] = useState("");
  const voices = useSystemVoices();

  useEffect(() => {
    invoke<SettingsView>("get_settings").then((loaded) => {
      setView(loaded);
      setDraft(loaded.settings);
    });
  }, []);

  if (!view || !draft) return null;

  const update = (change: (settings: Settings) => Settings) => setDraft(change(structuredClone(draft)));

  async function save() {
    try {
      const saved = await invoke<SettingsView>("save_settings", {
        request: { settings: draft, chatKey: chatKey || null, transcriptionKey: transcriptionKey || null },
      });
      setView(saved);
      setChatKey("");
      setTranscriptionKey("");
      setStatus("Saved");
    } catch (error) {
      setStatus(`Couldn't save: ${error}`);
    }
  }

  function previewVoice() {
    speechSynthesis.cancel();
    const utterance = new SpeechSynthesisUtterance("hey, I'm flitty. ask me anything about your screen.");
    const voice = voices.find((candidate) => candidate.name === draft!.voice.voiceName);
    if (voice) utterance.voice = voice;
    speechSynthesis.speak(utterance);
  }

  return (
    <main>
      <header>
        <h1>Flitty</h1>
        <p>Hold <kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>Space</kbd>, ask out loud, let go.</p>
      </header>

      <section>
        <h2>AI model</h2>
        <label>
          Provider
          <select
            value={presetName(CHAT_PRESETS, draft.chat.baseUrl)}
            onChange={(event) => {
              const preset = CHAT_PRESETS.find((candidate) => candidate.name === event.target.value);
              if (preset) update((settings) => ({ ...settings, chat: { api: preset.api, baseUrl: preset.baseUrl, model: preset.model } }));
            }}
          >
            {CHAT_PRESETS.map((preset) => <option key={preset.name}>{preset.name}</option>)}
            <option>Custom</option>
          </select>
        </label>
        <label>
          API style
          <select value={draft.chat.api} onChange={(event) => update((settings) => ({ ...settings, chat: { ...settings.chat, api: event.target.value as ChatApi } }))}>
            <option value="anthropic">Anthropic Messages</option>
            <option value="open-ai-compatible">OpenAI-compatible</option>
          </select>
        </label>
        <label>
          Base URL
          <input value={draft.chat.baseUrl} onChange={(event) => update((settings) => ({ ...settings, chat: { ...settings.chat, baseUrl: event.target.value } }))} />
        </label>
        <label>
          Model
          <input value={draft.chat.model} onChange={(event) => update((settings) => ({ ...settings, chat: { ...settings.chat, model: event.target.value } }))} />
        </label>
        <label>
          API key
          <input type="password" autoComplete="off" value={chatKey} placeholder={view.hasChatKey ? "Saved. Type to replace." : "Paste your key"} onChange={(event) => setChatKey(event.target.value)} />
        </label>
      </section>

      <section>
        <h2>Speech to text</h2>
        <label>
          Provider
          <select
            value={presetName(TRANSCRIPTION_PRESETS, draft.transcription.baseUrl)}
            onChange={(event) => {
              const preset = TRANSCRIPTION_PRESETS.find((candidate) => candidate.name === event.target.value);
              if (preset) update((settings) => ({ ...settings, transcription: { baseUrl: preset.baseUrl, model: preset.model } }));
            }}
          >
            {TRANSCRIPTION_PRESETS.map((preset) => <option key={preset.name}>{preset.name}</option>)}
            <option>Custom</option>
          </select>
        </label>
        <label>
          Base URL
          <input value={draft.transcription.baseUrl} onChange={(event) => update((settings) => ({ ...settings, transcription: { ...settings.transcription, baseUrl: event.target.value } }))} />
        </label>
        <label>
          API key
          <input
            type="password"
            autoComplete="off"
            value={transcriptionKey}
            placeholder={view.hasTranscriptionKey ? "Saved. Type to replace." : "Optional: uses the AI model key if empty"}
            onChange={(event) => setTranscriptionKey(event.target.value)}
          />
        </label>
      </section>

      <section>
        <h2>Voice</h2>
        <label>
          System voice
          <select value={draft.voice.voiceName} onChange={(event) => update((settings) => ({ ...settings, voice: { ...settings.voice, voiceName: event.target.value } }))}>
            <option value="">Best available</option>
            {voices.map((voice) => <option key={voice.name} value={voice.name}>{voice.name}</option>)}
          </select>
        </label>
        <div className="row">
          <label className="inline">
            <input type="checkbox" checked={draft.voice.muted} onChange={(event) => update((settings) => ({ ...settings, voice: { ...settings.voice, muted: event.target.checked } }))} />
            Don't speak replies
          </label>
          <button className="secondary" onClick={previewVoice}>Preview</button>
        </div>
      </section>

      <footer>
        <span className="status">{status}</span>
        <button onClick={save}>Save</button>
      </footer>
    </main>
  );
}

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <Panel />
  </StrictMode>,
);
