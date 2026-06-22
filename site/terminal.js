const scenes = [
  {
    id: "launch",
    nav: "launch",
    label: "[INFO] launch surface",
    title: "start from zero",
    statusLeft: "[PLAN] policy-gated",
    statusMid: "[INFO] foundation layer",
    lines: [
      "rz0",
      "",
      "runtime.zero",
      "safe system management foundation",
      "",
      "[INFO] loading core.policy",
      "[INFO] loading core.registry",
      "[PLAN] no mutation without explicit approval",
      "[PLAN] future TUI design reference only"
    ]
  },
  {
    id: "home",
    nav: "home",
    label: "[INFO] pre-alpha foundation",
    title: "operator home",
    statusLeft: "[OK] report-first",
    statusMid: "[DRY-RUN] default posture",
    lines: [
      "foundation layer",
      "safe system management",
      "",
      "[OK] report-first posture",
      "[OK] dry-run-first workflow",
      "[PLAN] modules available for inspection",
      "[QUARANTINE] reversible isolation model",
      "",
      "status: pre-alpha"
    ]
  },
  {
    id: "palette",
    nav: "palette",
    label: "[PLAN] command palette",
    title: "choose intent first",
    statusLeft: "[PLAN] inspect before action",
    statusMid: "[INFO] keyboard-native",
    lines: [
      "> doctor",
      "  modules",
      "  scan --dry-run",
      "  modules validate",
      "  store plan",
      "  quarantine review",
      "",
      "selection changes intent, not system state"
    ]
  },
  {
    id: "registry",
    nav: "modules",
    label: "[INFO] module registry",
    title: "manifest before module",
    statusLeft: "[PLAN] explicit modules",
    statusMid: "[INFO] no execution",
    lines: [
      "modules",
      "core.cli          present",
      "core.policy       present",
      "core.registry     present",
      "",
      "validate manifest",
      "sha256            required",
      "remote code       not allowed",
      "execution         not allowed during validation"
    ]
  },
  {
    id: "dryrun",
    nav: "dry-run",
    label: "[DRY-RUN] plan-first behavior",
    title: "scan without mutation",
    statusLeft: "[DRY-RUN] writes disabled",
    statusMid: "[SKIP] untouched",
    lines: [
      "scan --dry-run",
      "",
      "[PLAN] inspect target paths",
      "[PLAN] report candidate changes",
      "[SKIP] no files modified",
      "[SKIP] no installs performed",
      "[SKIP] no credentials touched",
      "[SKIP] no browser sessions touched"
    ]
  },
  {
    id: "restore",
    nav: "restore",
    label: "[QUARANTINE] reversible model",
    title: "restore path first",
    statusLeft: "[QUARANTINE] reversible",
    statusMid: "[BLOCKED] approval required",
    lines: [
      "quarantine review",
      "",
      "[QUARANTINE] isolated candidates",
      "[PLAN] receipt path recorded",
      "[PLAN] restore path available",
      "[BLOCKED] delete requires explicit approval",
      "",
      "github  docs  safety model"
    ]
  }
];

const root = document.documentElement;
const stage = document.querySelector(".scroll-stage");
const frame = document.querySelector("[data-tui-frame]");
const title = document.querySelector("[data-scene-title]");
const label = document.querySelector("[data-scene-label]");
const output = document.querySelector("[data-scene-output]");
const counter = document.querySelector("[data-scene-counter]");
const statusLeft = document.querySelector("[data-status-left]");
const statusMid = document.querySelector("[data-status-mid]");
const footer = document.querySelector("[data-footer-links]");
const navItems = Array.from(document.querySelectorAll("[data-nav-item]"));
const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
const narrowViewport = window.matchMedia("(max-width: 780px)").matches;
const layoutEnd = 0.27;
const sceneStart = 0.29;

let activeScene = -1;
let typingTimer = 0;

function clamp(value, min, max) {
  return Math.min(max, Math.max(min, value));
}

function progressForStage() {
  if (!stage) return 0;
  const rect = stage.getBoundingClientRect();
  const distance = rect.height - window.innerHeight;
  if (distance <= 0) return 1;
  return clamp(-rect.top / distance, 0, 1);
}

function setVars(layoutProgress, scrollProgress) {
  const reveal = clamp((layoutProgress - 0.12) / 0.16, 0, 1);
  const footerReveal = clamp((scrollProgress - 0.84) / 0.12, 0, 1);
  root.style.setProperty("--title-scale", String(1 - layoutProgress * 0.55));
  root.style.setProperty("--title-y", `${-layoutProgress * 118}px`);
  root.style.setProperty("--terminal-opacity", String(reveal));
  root.style.setProperty("--terminal-y", `${34 - reveal * 34}px`);
  root.style.setProperty("--terminal-scale", String(0.94 + reveal * 0.06));
  root.style.setProperty("--footer-opacity", String(footerReveal));
  root.style.setProperty("--scene-progress", String(scrollProgress));
}

function sceneProgressFor(scrollProgress) {
  return clamp((scrollProgress - sceneStart) / (1 - sceneStart), 0, 1);
}

function sceneIndexFor(progress) {
  if (progress < 0.18) return 0;
  if (progress < 0.34) return 1;
  if (progress < 0.49) return 2;
  if (progress < 0.64) return 3;
  if (progress < 0.79) return 4;
  return 5;
}

function typeLines(text) {
  if (!output) return;
  window.clearInterval(typingTimer);
  if (reduceMotion || narrowViewport) {
    output.textContent = text;
    return;
  }
  output.textContent = "";
  let cursor = 0;
  typingTimer = window.setInterval(() => {
    output.textContent = text.slice(0, cursor);
    cursor += 5;
    if (cursor > text.length) {
      output.textContent = text;
      window.clearInterval(typingTimer);
    }
  }, 9);
}

function renderScene(index) {
  if (!scenes[index] || index === activeScene) return;
  activeScene = index;
  const scene = scenes[index];
  if (title) title.textContent = scene.title;
  if (label) label.textContent = scene.label;
  if (counter) counter.textContent = `${String(index + 1).padStart(2, "0")} / ${String(scenes.length).padStart(2, "0")}`;
  if (statusLeft) statusLeft.textContent = scene.statusLeft;
  if (statusMid) statusMid.textContent = scene.statusMid;
  navItems.forEach((item) => item.classList.toggle("is-active", item.textContent.trim() === scene.nav));
  typeLines(`${scene.lines.join("\n")}\n`);
}

function renderDormant() {
  if (activeScene === -2) return;
  activeScene = -2;
  window.clearInterval(typingTimer);
  if (title) title.textContent = "";
  if (label) label.textContent = "";
  if (counter) counter.textContent = "00 / 06";
  if (statusLeft) statusLeft.textContent = "";
  if (statusMid) statusMid.textContent = "";
  if (output) output.textContent = "";
  navItems.forEach((item) => item.classList.remove("is-active"));
}

function update() {
  if (reduceMotion || narrowViewport) {
    setVars(1, 1);
    renderScene(0);
    return;
  }

  const scrollProgress = progressForStage();
  const layoutProgress = clamp(scrollProgress / layoutEnd, 0, 1);
  setVars(layoutProgress, scrollProgress);

  if (scrollProgress < sceneStart) {
    renderDormant();
    return;
  }

  renderScene(sceneIndexFor(sceneProgressFor(scrollProgress)));
}

function syncFooterFocus() {
  if (!footer) return;
  footer.addEventListener("focusin", () => {
    root.style.setProperty("--footer-opacity", "1");
  });
}

window.addEventListener("scroll", update, { passive: true });
window.addEventListener("resize", update);
syncFooterFocus();
if (frame) frame.setAttribute("aria-hidden", "true");
update();
