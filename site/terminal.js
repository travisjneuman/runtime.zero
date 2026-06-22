const scenes = [
  {
    badge: "REAL",
    lines: [
      "$ rz0 --version",
      "runtime.zero rz0 0.1.0",
      "System Management Toolkit",
      "",
      "$ rz0 doctor",
      "status: phase-1 bootstrap",
      "command: rz0",
      "version: 0.1.0",
      "os: windows | macos | linux",
      "arch: x86_64 | aarch64",
      "safety: report-first / dry-run-first / quarantine-first",
      "mutation_capability: disabled",
      "cloudflare_automation: not configured",
      "github_actions: not configured"
    ]
  },
  {
    badge: "REAL",
    lines: [
      "$ rz0 doctor",
      "status: phase-1 bootstrap",
      "command: rz0",
      "version: 0.1.0",
      "os: windows | macos | linux",
      "arch: x86_64 | aarch64",
      "safety: report-first / dry-run-first / quarantine-first",
      "mutation_capability: disabled",
      "cloudflare_automation: not configured",
      "github_actions: not configured"
    ]
  },
  {
    badge: "REAL",
    lines: [
      "$ rz0 modules",
      "core.brand         active    centralized build-time name and metadata",
      "core.cli           active    safe command parser and Phase 1 help surface",
      "core.doctor        active    read-only local runtime diagnostics",
      "core.scan-plan     stub      dry-run-only scan placeholder",
      "platform.windows   planned   Windows adapter for packages, registry, services, tasks, and AppData",
      "modules.update     planned   installed-only update orchestration",
      "modules.uninstall  planned   manager-native uninstall orchestration",
      "modules.leftovers  planned   report-first leftover classification and quarantine planning"
    ]
  },
  {
    badge: "REAL",
    lines: [
      "$ rz0 scan --dry-run",
      "runtime.zero scan plan",
      "",
      "mode: dry-run",
      "mutation_capability: disabled",
      "result: no system changes were attempted",
      "next: platform adapters will add read-only inventory in a later phase"
    ]
  },
  {
    badge: "PLANNED",
    lines: [
      "$ rz0 modules info first-party/windows-inventory",
      "status: planned",
      "publisher: runtime.zero first-party",
      "risk: read-only",
      "mutates_system: false",
      "outputs: normalized local inventory evidence"
    ]
  },
  {
    badge: "PLANNED",
    lines: [
      "$ rz0 modules install first-party/windows-inventory --dry-run",
      "plan: download first-party module manifest",
      "verify: publisher + checksum + signature",
      "compatibility: pending",
      "result: no module installed in dry-run"
    ]
  },
  {
    badge: "LOCKED",
    lines: [
      "$ rz0 run leftovers --quarantine --dry-run",
      "module: first-party/leftovers",
      "status: not installed",
      "risk: destructive-gated",
      "required: dry-run report + quarantine manifest + confirmation",
      "result: blocked safely",
      "",
      "$ rz0",
      "foundation: ready",
      "modules: user-selected",
      "trust: explicit"
    ]
  }
];

const terminal = document.querySelector("#terminal-output");
const sceneCards = Array.from(document.querySelectorAll(".scene"));
const reduceMotion = window.matchMedia("(prefers-reduced-motion: reduce)").matches;
let activeScene = -1;
let typingTimer = 0;

function renderScene(index) {
  if (!terminal || !scenes[index] || activeScene === index) {
    return;
  }

  activeScene = index;
  window.clearInterval(typingTimer);

  const scene = scenes[index];
  const text = `${scene.lines.join("\n")}\n`;
  terminal.dataset.badge = scene.badge;

  if (reduceMotion || index === 0) {
    terminal.textContent = text;
    return;
  }

  terminal.textContent = "";
  let cursor = 0;
  typingTimer = window.setInterval(() => {
    terminal.textContent = text.slice(0, cursor);
    cursor += 3;
    if (cursor > text.length) {
      terminal.textContent = text;
      window.clearInterval(typingTimer);
    }
  }, 8);
}

const observer = new IntersectionObserver(
  (entries) => {
    const visible = entries
      .filter((entry) => entry.isIntersecting)
      .sort((a, b) => b.intersectionRatio - a.intersectionRatio)[0];

    if (!visible) {
      return;
    }

    const sceneIndex = Number(visible.target.getAttribute("data-scene"));
    sceneCards.forEach((card) => card.classList.toggle("is-active", card === visible.target));
    renderScene(sceneIndex);
  },
  { rootMargin: "-35% 0px -35% 0px", threshold: [0.2, 0.45, 0.7] }
);

sceneCards.forEach((card) => observer.observe(card));
renderScene(0);
