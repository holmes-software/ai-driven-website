import { escapeHtml, normalizeTheme } from "./util";

interface Experience {
  title: string;
  company: string;
  start: string;
  end: string | null;
  achievements: string[];
}

interface Project {
  title: string;
  description: string;
  image: string;
  link: string;
}

interface Profile {
  about: { description: string; profile_pic: string };
  experience: Experience[];
  projects: Project[];
}

const $ = <T extends Element = Element>(sel: string) =>
  document.querySelector(sel) as T | null;

async function fetchProfile(): Promise<Profile> {
  const res = await fetch("/api/profile");
  if (!res.ok) throw new Error(`profile fetch failed: ${res.status}`);
  return res.json();
}

function renderProfile(profile: Profile) {
  const pic = $<HTMLImageElement>(".about .profile-pic");
  if (pic) pic.src = profile.about.profile_pic;
  const desc = $(".about .description");
  if (desc) desc.textContent = profile.about.description;

  const expList = $(".experience-list");
  if (expList) {
    expList.innerHTML = profile.experience
      .map(
        (e) => `
          <li class="experience-item">
            <time class="experience-dates">${escapeHtml(e.start)} — ${
              e.end ? escapeHtml(e.end) : "Present"
            }</time>
            <div class="experience-body">
              <h3 class="experience-role">${escapeHtml(e.title)}</h3>
              <p class="experience-company">${escapeHtml(e.company)}</p>
              <ul class="experience-achievements">
                ${e.achievements.map((a) => `<li>${escapeHtml(a)}</li>`).join("")}
              </ul>
            </div>
          </li>`,
      )
      .join("");
  }

  const grid = $(".projects-grid");
  if (grid) {
    grid.innerHTML = profile.projects
      .map(
        (p) => `
          <a class="project-card" href="${encodeURI(p.link)}" target="_blank" rel="noopener">
            <img src="${encodeURI(p.image)}" alt="${escapeHtml(p.title)}" />
            <div class="project-body">
              <h3 class="project-title">${escapeHtml(p.title)}</h3>
              <p class="project-description">${escapeHtml(p.description)}</p>
            </div>
          </a>`,
      )
      .join("");
  }
}

async function loadStyles(theme: string) {
  // Get the HTML before any themes are changed to be marked as "busy"
  const html = getStylelessHtml();
  const themeInput = $<HTMLInputElement>("#theme-input");
  const themeApply = $<HTMLButtonElement>("#theme-apply");
  setControlsBusy(true, themeInput, themeApply);
  try {
    const res = await fetch("/api/styles", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ html, theme }),
    });
    if (!res.ok) throw new Error(`styles fetch failed: ${res.status}`);
    const { css } = (await res.json()) as { css: string; source: string };
    applyCss(css);
  } finally {
    setControlsBusy(false, themeInput, themeApply);
  }
}

function getStylelessHtml() {
  const clone = document.documentElement.cloneNode(true) as HTMLElement;
  clone
    .querySelectorAll("style, link[rel='stylesheet']")
    .forEach((n) => n.remove());
  return clone.outerHTML;
}

function setControlsBusy(
  busy: boolean,
  input: HTMLInputElement | null,
  button: HTMLButtonElement | null,
) {
  if (input) {
    input.disabled = busy;
    input.setAttribute("aria-busy", String(busy));
    if (busy) {
      // Stash the original placeholder so we can restore it after loading.
      input.dataset.defaultPlaceholder = input.placeholder;
      input.value = "";
      input.placeholder = "Loading...";
    } else if (input.dataset.defaultPlaceholder !== undefined) {
      input.placeholder = input.dataset.defaultPlaceholder;
      delete input.dataset.defaultPlaceholder;
    }
  }
  if (button) {
    button.disabled = busy;
    button.setAttribute("aria-busy", String(busy));
  }
}

function applyCss(css: string) {
  const el = document.getElementById(
    "ai-styles",
  ) as HTMLStyleElement | null;
  el?.remove();
  const new_el = document.createElement("style");
  new_el.id = "ai-styles";
  new_el.textContent = css;
  document.head.appendChild(new_el);
}

function flashInvalid(el: HTMLInputElement) {
  // Restart the animation by removing and re-adding the class on the next frame.
  el.classList.remove("invalid");
  // Force reflow so the animation re-triggers reliably.
  void el.offsetWidth;
  el.classList.add("invalid");
  window.setTimeout(() => el.classList.remove("invalid"), 450);
}

function wireControls() {
  const themeInput = $<HTMLInputElement>("#theme-input");
  const themeApply = $<HTMLButtonElement>("#theme-apply");

  themeInput?.addEventListener("input", () => {
    if (!themeInput) return;
    const before = themeInput.value;
    const after = normalizeTheme(before);
    if (after !== before) {
      themeInput.value = after;
      flashInvalid(themeInput);
    }
  });

  themeApply?.addEventListener("click", () => {
    const theme = normalizeTheme(themeInput?.value || "");
    if (!theme) {
      if (themeInput) flashInvalid(themeInput);
      return;
    }
    loadStyles(theme).catch((e) => console.error(e));
  });

  themeInput?.addEventListener("keydown", (e) => {
    if (e.key === "Enter") themeApply?.click();
  });

  $<HTMLButtonElement>("#download-resume")?.addEventListener("click", () => {
    window.location.href = "/api/resume.pdf";
  });
}

async function main() {
  wireControls();
  const profile = await fetchProfile();
  renderProfile(profile);
}

main().catch((e) => {
  console.error(e);
  document.body.insertAdjacentHTML(
    "afterbegin",
    `<pre style="color:red;padding:1rem">Failed to load: ${escapeHtml(
      String(e),
    )}</pre>`,
  );
});
