// Speaks reply sentences in order with the system's text-to-speech voices.

export type Speaker = {
  say(text: string): void;
  stop(): void;
  markReplyComplete(): void;
  setPreferredVoice(name: string): void;
};

/** Natural-sounding voices first, then any English voice. */
function pickVoice(preferredName: string): SpeechSynthesisVoice | undefined {
  const voices = speechSynthesis.getVoices();
  if (preferredName) {
    const chosen = voices.find((voice) => voice.name === preferredName);
    if (chosen) return chosen;
  }
  const english = voices.filter((voice) => voice.lang.toLowerCase().startsWith("en"));
  return (
    english.find((voice) => /natural|neural|online/i.test(voice.name)) ??
    english.find((voice) => voice.localService) ??
    english[0] ??
    voices[0]
  );
}

export function createSpeaker(onFinishedReply: () => void): Speaker {
  let preferredVoice = "";
  let pending = 0;
  let replyComplete = false;

  function finishIfDone() {
    if (replyComplete && pending === 0) {
      replyComplete = false;
      onFinishedReply();
    }
  }

  return {
    say(text) {
      const utterance = new SpeechSynthesisUtterance(text);
      const voice = pickVoice(preferredVoice);
      if (voice) utterance.voice = voice;
      utterance.rate = 1.05;
      pending += 1;
      const done = () => {
        pending = Math.max(0, pending - 1);
        finishIfDone();
      };
      utterance.onend = done;
      utterance.onerror = done;
      speechSynthesis.speak(utterance);
    },
    stop() {
      pending = 0;
      replyComplete = false;
      speechSynthesis.cancel();
    },
    markReplyComplete() {
      replyComplete = true;
      finishIfDone();
    },
    setPreferredVoice(name) {
      preferredVoice = name;
    },
  };
}
