import { StrictMode, useEffect, useState } from "react";
import { createRoot } from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import "./panel.css";

type Settings = { chatModel: string; transcriptionModel: string; voiceName: string; muted: boolean };
type SettingsView = { settings: Settings; hasKey: boolean };

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
  const [apiKey, setApiKey] = useState("");
  const [status, setStatus] = useState("");
  const [testing, setTesting] = useState(false);
  const voices = useSystemVoices();

  useEffect(() => {
    invoke<SettingsView>("get_settings").then((loaded) => {
      setView(loaded);
      setDraft(loaded.settings);
    });
  }, []);

  if (!view || !draft) return null;

  async function save(): Promise<boolean> {
    try {
      const saved = await invoke<SettingsView>("save_settings", { request: { settings: draft, apiKey: apiKey || null } });
      setView(saved);
      setApiKey("");
      setStatus("Saved");
      return true;
    } catch (error) {
      setStatus(`Couldn't save: ${error}`);
      return false;
    }
  }

  async function testKey() {
    setTesting(true);
    try {
      if (!(await save())) return;
      await invoke("test_key");
      setStatus("✓ OpenAI accepted your key");
    } catch (error) {
      setStatus(`✗ ${error}`);
    } finally {
      setTesting(false);
    }
  }

  function previewVoice() {
    speechSynthesis.cancel();
    const utterance = new SpeechSynthesisUtterance("hi, I'm HeyFlitty. ask me anything about your screen.");
    const voice = voices.find((candidate) => candidate.name === draft!.voiceName);
    if (voice) utterance.voice = voice;
    speechSynthesis.speak(utterance);
  }

  return (
    <main>
      <header>
        <h1>HeyFlitty</h1>
        <p>Hold <kbd>Ctrl</kbd> + <kbd>Alt</kbd> + <kbd>Space</kbd>, ask out loud, let go.</p>
      </header>

      <section>
        <h2>OpenAI</h2>
        <label>
          API key
          <input type="password" autoComplete="off" value={apiKey} placeholder={view.hasKey ? "Saved. Type to replace." : "sk-..."} onChange={(event) => setApiKey(event.target.value)} />
        </label>
        <label>
          Model
          <input value={draft.chatModel} onChange={(event) => setDraft({ ...draft, chatModel: event.target.value })} />
        </label>
      </section>

      <section>
        <h2>Voice</h2>
        <label>
          System voice
          <select value={draft.voiceName} onChange={(event) => setDraft({ ...draft, voiceName: event.target.value })}>
            <option value="">Best English voice</option>
            {voices.map((voice) => <option key={voice.name} value={voice.name}>{voice.name}</option>)}
          </select>
        </label>
        <div className="row">
          <label className="inline">
            <input type="checkbox" checked={draft.muted} onChange={(event) => setDraft({ ...draft, muted: event.target.checked })} />
            Don't speak replies
          </label>
          <button className="secondary" onClick={previewVoice}>Preview</button>
        </div>
      </section>

      <footer>
        <span className="status">{status}</span>
        <button className="secondary" onClick={testKey} disabled={testing}>{testing ? "Testing…" : "Test key"}</button>
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
