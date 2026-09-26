import fs from "node:fs";
import path from "node:path";

const webdriverUrl = process.env.WEBDRIVER_URL ?? "http://127.0.0.1:4444";
const appBinary = path.resolve(process.argv[2] ?? "");
const projectRoot = path.resolve(process.argv[3] ?? "");
const evidencePath = path.resolve(process.argv[4] ?? "desktop-e2e-evidence.json");
const ELEMENT_KEY = "element-6066-11e4-a52e-4f735466cecf";

if (!appBinary || !projectRoot) {
  throw new Error("usage: node desktop-e2e.mjs <app-binary> <project-root> [evidence-json]");
}

const evidence = {
  candidate: process.env.CANDIDATE_SHA ?? process.env.GITHUB_SHA ?? "local",
  platform: process.platform,
  node: process.version,
  appBinary,
  projectRoot,
  webdriverUrl,
  startedAt: new Date().toISOString(),
  steps: [],
};

function record(step, detail = "") {
  evidence.steps.push({ step, detail, at: new Date().toISOString() });
  console.log(`[desktop-e2e] ${step}${detail ? `: ${detail}` : ""}`);
}

function sleep(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

async function request(method, endpoint, body, { allowError = false } = {}) {
  const response = await fetch(`${webdriverUrl}${endpoint}`, {
    method,
    headers: body === undefined ? undefined : { "content-type": "application/json" },
    body: body === undefined ? undefined : JSON.stringify(body),
  });
  const text = await response.text();
  let payload = {};
  if (text) {
    try {
      payload = JSON.parse(text);
    } catch {
      payload = { value: text };
    }
  }
  const value = payload?.value;
  const protocolError =
    !response.ok ||
    (value && typeof value === "object" && typeof value.error === "string");
  if (protocolError && !allowError) {
    throw new Error(
      `WebDriver ${method} ${endpoint} failed (${response.status}): ${JSON.stringify(payload)}`,
    );
  }
  return { ok: !protocolError, status: response.status, payload };
}

async function waitForDriver(timeoutMs = 30_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      const result = await request("GET", "/status", undefined, { allowError: true });
      if (result.status < 500) return;
    } catch {
      // driver not listening yet
    }
    await sleep(250);
  }
  throw new Error("tauri-driver did not become ready");
}

function elementId(result) {
  const value = result?.payload?.value;
  if (!value || typeof value !== "object") return null;
  return value[ELEMENT_KEY] ?? value.ELEMENT ?? null;
}

async function elements(xpath) {
  const result = await request(
    "POST",
    `/session/${sessionId}/elements`,
    { using: "xpath", value: xpath },
    { allowError: true },
  );
  const values = result?.payload?.value;
  if (!result.ok || !Array.isArray(values)) return [];
  return values.map((item) => item?.[ELEMENT_KEY] ?? item?.ELEMENT).filter(Boolean);
}

async function displayed(id) {
  const result = await request(
    "GET",
    `/session/${sessionId}/element/${id}/displayed`,
    undefined,
    { allowError: true },
  );
  return result.ok && result.payload?.value === true;
}

async function waitElement(xpath, timeoutMs = 20_000) {
  const deadline = Date.now() + timeoutMs;
  let lastCount = 0;
  while (Date.now() < deadline) {
    const ids = await elements(xpath);
    lastCount = ids.length;
    for (const id of ids) {
      if (await displayed(id)) return id;
    }
    await sleep(200);
  }
  throw new Error(`timed out waiting for visible xpath: ${xpath} (last count ${lastCount})`);
}

async function waitGone(xpath, timeoutMs = 20_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const ids = await elements(xpath);
    let anyVisible = false;
    for (const id of ids) {
      if (await displayed(id)) {
        anyVisible = true;
        break;
      }
    }
    if (!anyVisible) return;
    await sleep(200);
  }
  throw new Error(`timed out waiting for xpath to disappear: ${xpath}`);
}

async function click(xpath, timeoutMs = 20_000) {
  const deadline = Date.now() + timeoutMs;
  let lastError = "";
  while (Date.now() < deadline) {
    const ids = await elements(xpath);
    for (const id of ids) {
      if (await displayed(id)) {
        const result = await request(
          "POST",
          `/session/${sessionId}/element/${id}/click`,
          {},
          { allowError: true },
        );
        if (result.ok) return id;
        lastError = JSON.stringify(result.payload);
      }
    }
    await sleep(200);
  }
  throw new Error(`timed out clicking xpath: ${xpath}; last WebDriver error: ${lastError}`);
}

async function enterCellEdit(xpath, timeoutMs = 20_000) {
  await waitElement(xpath, timeoutMs);
  await execute(
    `const cell = document.querySelector('[role="gridcell"][aria-label^="new record id"]');
     if (!cell) return false;
     cell.scrollIntoView({ block: "center", inline: "nearest" });
     return true;`,
  );
  await sleep(200);
  const focused = await execute(
    `const cell = document.querySelector('[role="gridcell"][aria-label^="new record id"]');
     if (!cell) return false;
     cell.focus();
     cell.dispatchEvent(new KeyboardEvent("keydown", { key: "Enter", code: "Enter", bubbles: true }));
     return true;`,
  );
  if (!focused.ok || focused.value !== true) throw new Error("could not focus the new-row record-id cell");
  await sleep(300);
}

async function fill(xpath, value, timeoutMs) {
  const id = await waitElement(xpath, timeoutMs);
  await request("POST", `/session/${sessionId}/element/${id}/clear`, {});
  await request("POST", `/session/${sessionId}/element/${id}/value`, {
    text: value,
    value: [...value],
  });
  return id;
}

async function execute(script, args = []) {
  const result = await request(
    "POST",
    `/session/${sessionId}/execute/sync`,
    { script, args },
    { allowError: true },
  );
  if (!result.ok) return { ok: false, value: null };
  return { ok: true, value: result.payload?.value };
}

async function selectNative(label, option, timeoutMs = 20_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    const selected = await execute(
      `const label = arguments[0];
       const wanted = arguments[1];
       const select = [...document.querySelectorAll("select")]
         .find((node) => node.getAttribute("aria-label") === label);
       if (!select || ![...select.options].some((candidate) => candidate.value === wanted)) return false;
       select.value = wanted;
       select.dispatchEvent(new Event("input", { bubbles: true }));
       select.dispatchEvent(new Event("change", { bubbles: true }));
       return true;`,
      [label, option],
    );
    if (selected.ok && selected.value === true) return;
    await sleep(200);
  }
  throw new Error(`timed out selecting native option ${option} for ${label}`);
}

async function focusSourceRoot() {
  const focused = await execute(
    `const root = document.querySelector("button.tree-root-label");
     if (!root) return false;
     root.focus();
     root.dispatchEvent(new FocusEvent("focusin", { bubbles: true }));
     root.dispatchEvent(new MouseEvent("mousedown", { bubbles: true }));
     if (root.getAttribute("aria-expanded") !== "true") root.click();
     return document.activeElement === root;`,
  );
  if (!focused.ok || focused.value !== true) throw new Error("could not focus the configured source root");
  await sleep(200);
}

async function waitText(text, timeoutMs = 20_000) {
  return waitElement(`//*[contains(normalize-space(.), ${xpathLiteral(text)})]`, timeoutMs);
}

function findYamlDocuments(root, requiredFragments) {
  const matches = [];
  const visit = (dir) => {
    if (!fs.existsSync(dir)) return;
    for (const entry of fs.readdirSync(dir, { withFileTypes: true })) {
      const full = path.join(dir, entry.name);
      if (entry.isDirectory()) visit(full);
      else if (/\.ya?ml$/i.test(entry.name)) {
        const source = fs.readFileSync(full, "utf8");
        if (requiredFragments.every((fragment) => source.includes(fragment))) matches.push(full);
      }
    }
  };
  visit(root);
  return matches;
}

async function waitUniqueYamlDocument(root, requiredFragments, timeoutMs = 20_000) {
  const deadline = Date.now() + timeoutMs;
  let matches = [];
  while (Date.now() < deadline) {
    matches = findYamlDocuments(root, requiredFragments);
    if (matches.length === 1) return matches[0];
    if (matches.length > 1) break;
    await sleep(200);
  }
  throw new Error(
    `expected exactly one YAML document containing ${requiredFragments.join(" / ")} under ${root}, found ${matches.length}: ${matches.join(", ")}`,
  );
}

async function waitFileContains(filePath, fragment, timeoutMs = 20_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    try {
      if (fs.readFileSync(filePath, "utf8").includes(fragment)) return;
    } catch {
      // file may not exist yet
    }
    await sleep(200);
  }
  throw new Error(`timed out waiting for ${filePath} to contain ${fragment}`);
}

function xpathLiteral(value) {
  if (!value.includes("'")) return `'${value}'`;
  if (!value.includes('"')) return `"${value}"`;
  return `concat('${value.replaceAll("'", `', "'", '`)}')`;
}

async function screenshot() {
  if (!sessionId) return;
  try {
    const result = await request("GET", `/session/${sessionId}/screenshot`, undefined, {
      allowError: true,
    });
    if (result.ok && typeof result.payload?.value === "string") {
      fs.writeFileSync(
        path.join(path.dirname(evidencePath), "desktop-e2e-failure.png"),
        Buffer.from(result.payload.value, "base64"),
      );
    }
  } catch {
    // best-effort failure evidence
  }
}

fs.mkdirSync(path.dirname(projectRoot), { recursive: true });
fs.rmSync(projectRoot, { recursive: true, force: true });
fs.mkdirSync(path.dirname(evidencePath), { recursive: true });

let sessionId = "";
try {
  await waitForDriver();
  record("webdriver-ready");

  const session = await request("POST", "/session", {
    capabilities: {
      alwaysMatch: {
        "tauri:options": {
          application: appBinary,
        },
      },
    },
  });
  sessionId = session.payload?.value?.sessionId ?? session.payload?.sessionId ?? "";
  if (!sessionId) throw new Error(`missing WebDriver session id: ${JSON.stringify(session.payload)}`);
  record("desktop-session-started", sessionId);

  await click("//button[normalize-space(.)='Create Project']");
  await waitElement("//section[@aria-label='Create Project']");
  await fill("//*[@aria-label='Project destination']", projectRoot);
  await fill("//*[@aria-label='New project ID']", "desktop.evidence");
  await fill("//*[@aria-label='New project name']", "Desktop Evidence");
  await click("//section[@aria-label='Create Project']//button[normalize-space(.)='Create Project']");
  await waitElement("//aside[@aria-label='Explorer']", 30_000);
  record("project-created-through-gui");

  await focusSourceRoot();
  await click("//*[@aria-label='New source artifact']");
  await click("//*[@role='menuitem' and normalize-space(.)='Table']");
  await fill("//*[@aria-label='Filename (.yaml / .yml)']", "item-schema.yaml");
  await click("//*[@aria-label='Create']");
  await waitText("item-schema.yaml", 30_000);
  record("table-created-through-gui");

  await focusSourceRoot();
  await click("//*[@aria-label='New source artifact']");
  await click("//*[@role='menuitem' and normalize-space(.)='Data']");
  await selectNative("Existing Table", "item-schema");
  await fill("//*[@aria-label='Filename (.yaml / .yml)']", "items.yaml");
  await click("//*[@aria-label='Create']");
  await waitElement("//*[@aria-label='Add Row']", 30_000);
  const dataFile = await waitUniqueYamlDocument(projectRoot, ["kind: data", "table: item"], 30_000);
  record("data-source-created-through-gui", path.relative(projectRoot, dataFile));

  await click("//*[@aria-label='Add Row']");
  await enterCellEdit("//*[@role='gridcell' and starts-with(@aria-label, 'new record id')]");
  await sleep(200);
  await fill("//*[@role='textbox' and starts-with(@aria-label, 'new record id')]", "1001");
  await click("//section[contains(@class,'data-editor')]//button[normalize-space(.)='Save' and not(@disabled)]", 30_000);
  await waitFileContains(dataFile, "1001", 30_000);
  record("record-edited-and-saved-through-gui", path.relative(projectRoot, dataFile));

  await click("//button[normalize-space(.)='Settings']");
  await waitElement("//section[@aria-label='Project Settings']");
  await fill("//*[@aria-label='Profile name']", "prod");
  await click("//button[normalize-space(.)='Apply Profile to buffer' and not(@disabled)]");
  await fill("//*[@aria-label='Publish target path']", "delivery");
  await click("//button[normalize-space(.)='Apply Target to buffer' and not(@disabled)]");
  await click("//section[@aria-label='Configuration diff']//button[normalize-space(.)='Save Settings' and not(@disabled)]", 30_000);
  await waitElement("//section[@aria-label='Project Settings']//*[normalize-space(.)='Saved']", 30_000);
  await waitFileContains(path.join(projectRoot, "masterdata.toml"), "delivery", 30_000);
  record("settings-profile-and-publish-target-saved-through-gui");

  await click("//button[normalize-space(.)='Delivery']");
  await waitElement("//section[@aria-label='Build and Publish']");
  await click("//section[@aria-label='Build and Publish']//button[normalize-space(.)='Build saved input' and not(@disabled)]");
  await waitText("Build succeeded", 180_000);
  record("build-completed-through-gui");

  await click("//section[@aria-label='Build and Publish']//button[normalize-space(.)='Publish preview' and not(@disabled)]");
  await waitElement("//section[@aria-label='Publish preview']", 30_000);

  const delivery = path.join(projectRoot, "delivery");
  fs.mkdirSync(delivery, { recursive: true });
  const external = path.join(delivery, "External.g.cs");
  fs.writeFileSync(external, "external");
  fs.writeFileSync(
    path.join(delivery, ".masterdata-publish-manifest.json"),
    JSON.stringify({ version: 1, files: ["External.g.cs"] }),
  );
  record("external-publish-destination-change-injected");

  await click("//section[@aria-label='Publish preview']//button[normalize-space(.)='Confirm Publish' and not(@disabled)]");
  await waitText("E-PUBLISH-PREVIEW-STALE-DESTINATION", 30_000);
  if (fs.readFileSync(external, "utf8") !== "external") {
    throw new Error("stale Publish mutated the external managed file");
  }
  record("stale-publish-rejected-without-mutation");

  await click("//section[@aria-label='Build and Publish']//button[normalize-space(.)='Publish preview' and not(@disabled)]");
  await waitElement("//section[@aria-label='Publish preview']", 30_000);
  await click("//section[@aria-label='Publish preview']//button[normalize-space(.)='Confirm Publish' and not(@disabled)]");
  await waitText("Publish completed", 60_000);
  if (fs.existsSync(external)) {
    throw new Error("fresh Publish did not retire the reviewed managed external file");
  }
  if (!fs.existsSync(path.join(delivery, ".masterdata-publish-manifest.json"))) {
    throw new Error("fresh Publish did not write its manifest");
  }
  record("fresh-publish-completed-through-gui");

  evidence.status = "pass";
  evidence.finishedAt = new Date().toISOString();
  fs.writeFileSync(evidencePath, JSON.stringify(evidence, null, 2));
  console.log(JSON.stringify(evidence, null, 2));
} catch (error) {
  evidence.status = "fail";
  evidence.error = error instanceof Error ? error.stack ?? error.message : String(error);
  evidence.finishedAt = new Date().toISOString();
  await screenshot();
  fs.writeFileSync(evidencePath, JSON.stringify(evidence, null, 2));
  console.error(evidence.error);
  throw error;
} finally {
  if (sessionId) {
    await request("DELETE", `/session/${sessionId}`, undefined, { allowError: true }).catch(() => {});
  }
}
