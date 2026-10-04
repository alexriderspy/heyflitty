// The cursor buddy: follows the mouse, shows voice state, and flies to targets.
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { createSpeaker } from "./speech";

type Point = { x: number; y: number };
type CursorEvent = Point & { inside: boolean };
type PointTarget = Point & { label: string };
type VoiceState = "idle" | "listening" | "processing" | "responding";

const buddy = document.getElementById("buddy")!;
const pointer = buddy.querySelector<SVGElement>(".pointer")!;
const bubble = document.getElementById("bubble")!;
const status = document.getElementById("status")!;
const caption = document.getElementById("caption")!;
const captionHeard = caption.querySelector<HTMLElement>(".heard")!;
const captionSpoken = caption.querySelector<HTMLElement>(".spoken")!;
const CAPTION_WIDTH = 340;
const CAPTION_LINGER_MS = 2500;

// Sits beside the real cursor instead of under it.
const FOLLOW_OFFSET: Point = { x: 22, y: 18 };
const RESTING_ROTATION_DEGREES = -35;
const POINTING_HOLD_MS = 2500;

let mouse: CursorEvent = { x: 0, y: 0, inside: false };
let position: Point = { x: 0, y: 0 };
let mode: "follow" | "flying" | "pointing" = "follow";
let activeFlight: { cancelled: boolean } | null = null;

function render(rotationDegrees = RESTING_ROTATION_DEGREES) {
  buddy.style.transform = `translate(${position.x}px, ${position.y}px)`;
  // Caption sits below-right of the buddy, flipped left or above near the screen edges (the taskbar, say).
  const flip = position.x + 24 + CAPTION_WIDTH > window.innerWidth;
  const captionX = flip ? position.x - CAPTION_WIDTH - 12 : position.x + 24;
  const above = position.y + 30 + caption.offsetHeight > window.innerHeight - 8;
  const captionY = above ? position.y - caption.offsetHeight - 16 : position.y + 30;
  caption.style.transform = `translate(${Math.max(8, captionX)}px, ${Math.max(8, captionY)}px)`;
  // The label flips the same way so it never hangs off the screen.
  buddy.classList.toggle("near-bottom", position.y + 44 > window.innerHeight);
  buddy.classList.toggle("near-right", position.x + 18 + bubble.offsetWidth > window.innerWidth - 4);
  pointer.style.transform = `rotate(${rotationDegrees}deg)`;
  buddy.style.opacity = mouse.inside || mode !== "follow" ? "1" : "0";
}

function followPoint(): Point {
  return { x: mouse.x + FOLLOW_OFFSET.x, y: mouse.y + FOLLOW_OFFSET.y };
}

// Quadratic bezier arc with ease-in-out; the pointer turns along the path's tangent.
function fly(from: Point, to: Point): Promise<void> {
  if (activeFlight) activeFlight.cancelled = true;
  const flight = { cancelled: false };
  activeFlight = flight;
  const distance = Math.hypot(to.x - from.x, to.y - from.y);
  const durationMs = Math.min(1200, Math.max(500, distance * 0.9));
  const control: Point = { x: (from.x + to.x) / 2, y: Math.min(from.y, to.y) - Math.min(220, distance * 0.35) };
  const started = performance.now();

  return new Promise((resolve) => {
    function step(now: number) {
      if (flight.cancelled) return resolve();
      const linear = Math.min(1, (now - started) / durationMs);
      const t = linear < 0.5 ? 2 * linear * linear : 1 - Math.pow(-2 * linear + 2, 2) / 2;
      const u = 1 - t;
      position = {
        x: u * u * from.x + 2 * u * t * control.x + t * t * to.x,
        y: u * u * from.y + 2 * u * t * control.y + t * t * to.y,
      };
      const tangent = {
        x: 2 * u * (control.x - from.x) + 2 * t * (to.x - control.x),
        y: 2 * u * (control.y - from.y) + 2 * t * (to.y - control.y),
      };
      render((Math.atan2(tangent.y, tangent.x) * 180) / Math.PI + 90);
      if (linear < 1) requestAnimationFrame(step);
      else resolve();
    }
    requestAnimationFrame(step);
  });
}

listen<CursorEvent>("cursor", ({ payload }) => {
  mouse = payload;
  if (mode === "follow") {
    position = followPoint();
    render();
  }
});

let captionTimer: number | undefined;
function showCaption(sentence: string) {
  window.clearTimeout(captionTimer);
  captionSpoken.textContent = sentence;
  caption.classList.add("visible");
}
function hideCaptionSoon() {
  window.clearTimeout(captionTimer);
  captionTimer = window.setTimeout(() => {
    caption.classList.remove("visible");
    captionHeard.textContent = "";
    captionSpoken.textContent = "";
  }, CAPTION_LINGER_MS);
}

const speaker = createSpeaker(
  () => {
    // Speech queue drained after the reply finished: back to idle.
    if (status.className === "responding") status.className = "";
    hideCaptionSoon();
  },
  (voiceName) => showNotice(`the voice "${voiceName}" isn't installed; pick another in Settings`),
  showCaption,
);

async function refreshVoice() {
  const view = await invoke<{ settings: { voiceName: string; muted: boolean } }>("get_settings");
  speaker.setPreferredVoice(view.settings.voiceName);
  speaker.setMuted(view.settings.muted);
}
refreshVoice();
listen("settings-changed", refreshVoice);

listen<VoiceState>("voice-state", ({ payload }) => {
  status.className = payload === "idle" ? "" : payload;
});

listen<{ text: string }>("heard", ({ payload }) => {
  window.clearTimeout(captionTimer);
  captionHeard.textContent = payload.text;
  captionSpoken.textContent = "…";
  caption.classList.add("visible");
});
listen<{ text: string }>("speak", ({ payload }) => speaker.say(payload.text));
listen("stop-speech", () => {
  speaker.stop();
  caption.classList.remove("visible");
});
listen("response-complete", () => speaker.markReplyComplete());

let noticeTimer: number | undefined;
function showNotice(text: string) {
  bubble.textContent = text;
  bubble.classList.add("notice");
  bubble.style.opacity = "1";
  window.clearTimeout(noticeTimer);
  noticeTimer = window.setTimeout(() => {
    bubble.style.opacity = "0";
    bubble.classList.remove("notice");
  }, 6000);
}
listen<{ text: string }>("notice", ({ payload }) => showNotice(payload.text));

listen<boolean>("buddy-visibility", ({ payload }) => {
  buddy.classList.toggle("hidden", !payload);
});

listen<PointTarget>("point", async ({ payload }) => {
  mode = "flying";
  await fly({ ...position }, payload);
  mode = "pointing";
  render();
  bubble.classList.remove("notice");
  bubble.textContent = payload.label;
  bubble.style.opacity = payload.label ? "1" : "0";
  render();
  await new Promise((resolve) => setTimeout(resolve, POINTING_HOLD_MS));
  bubble.style.opacity = "0";
  mode = "flying";
  await fly({ ...position }, followPoint());
  mode = "follow";
});
