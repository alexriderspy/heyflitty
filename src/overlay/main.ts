// The cursor buddy: follows the mouse, shows voice state, and flies to targets.
import { listen } from "@tauri-apps/api/event";

type Point = { x: number; y: number };
type CursorEvent = Point & { inside: boolean };
type PointTarget = Point & { label: string };
type VoiceState = "idle" | "listening" | "processing" | "responding";

const buddy = document.getElementById("buddy")!;
const pointer = buddy.querySelector<SVGElement>(".pointer")!;
const bubble = document.getElementById("bubble")!;
const status = document.getElementById("status")!;

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

listen<VoiceState>("voice-state", ({ payload }) => {
  status.className = payload === "idle" || payload === "responding" ? "" : payload;
});

listen<boolean>("buddy-visibility", ({ payload }) => {
  buddy.classList.toggle("hidden", !payload);
});

listen<PointTarget>("point", async ({ payload }) => {
  mode = "flying";
  await fly({ ...position }, payload);
  mode = "pointing";
  render();
  bubble.textContent = payload.label;
  bubble.style.opacity = "1";
  await new Promise((resolve) => setTimeout(resolve, POINTING_HOLD_MS));
  bubble.style.opacity = "0";
  mode = "flying";
  await fly({ ...position }, followPoint());
  mode = "follow";
});
