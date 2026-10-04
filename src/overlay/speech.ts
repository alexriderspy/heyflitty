// Speaks reply sentences in order with the system's text-to-speech voices.

export type Speaker = {
  say(text: string): void;
  stop(): void;
  markReplyComplete(): void;
  setPreferredVoice(name: string): void;
  setMuted(muted: boolean): void;
};

/** With no voice chosen, prefer a natural-sounding English voice. */
function bestEnglishVoice(voices: SpeechSynthesisVoice[]): SpeechSynthesisVoice | undefined {
  const english = voices.filter((voice) => voice.lang.toLowerCase().startsWith("en"));
  return english.find((voice) => /natural|neural|online/i.test(voice.name)) ?? english.find((voice) => voice.localService) ?? english[0];
}

/** How long a muted caption stays up: roughly reading speed. */
const mutedCaptionMs = (text: string) => Math.max(1500, text.length * 55);

export function createSpeaker(
  onFinishedReply: () => void,
  onMissingVoice: (name: string) => void,
  onSentence: (text: string) => void,
): Speaker {
  let preferredVoice = "";
  let muted = false;
  let mutedQueue: Promise<void> = Promise.resolve();
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
      if (muted) {
        // No voice: show each sentence for about as long as it takes to read.
        pending += 1;
        mutedQueue = mutedQueue.then(async () => {
          onSentence(text);
          await new Promise((resolve) => setTimeout(resolve, mutedCaptionMs(text)));
          pending = Math.max(0, pending - 1);
          finishIfDone();
        });
        return;
      }
      const voices = speechSynthesis.getVoices();
      const voice = preferredVoice ? voices.find((candidate) => candidate.name === preferredVoice) : bestEnglishVoice(voices);
      if (preferredVoice && !voice) {
        onMissingVoice(preferredVoice);
        return;
      }
      const utterance = new SpeechSynthesisUtterance(text);
      if (voice) utterance.voice = voice;
      utterance.rate = 1.05;
      pending += 1;
      const done = () => {
        pending = Math.max(0, pending - 1);
        finishIfDone();
      };
      utterance.onstart = () => onSentence(text);
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
    setMuted(value) {
      muted = value;
    },
  };
}
