// Bootstrap the Dioxus WASM UI (external file so CSP can stay strict).



function setStatus(text) {
  const el = document.getElementById("boot-loader");
  if (el) {
    el.innerHTML =
      "<p style='padding:2rem;font-family:system-ui,sans-serif;color:#64748b'>" +
      text +
      "</p>";
  }
}

setStatus("Loading nx9-auth UI…");

import init from "/nx9_auth_ui.js";

// Call with no args: wasm-bindgen resolves nx9_auth_ui_bg.wasm next to
// nx9_auth_ui.js via import.meta.url (avoids the deprecated string form).
init()
  .then(() => {
    console.info("[nx9-auth-ui] WASM started");
    const el = document.getElementById("main");
    if (el) el.dataset.dioxusMounted = "1";
    const loader = document.getElementById("boot-loader");
    if (loader) loader.remove();
  })
  .catch((err) => {
    console.error("[nx9-auth-ui] Bootstrap failed", err);
    const loader = document.getElementById("boot-loader");
    if (loader) {
      loader.innerHTML = `
        <div style="max-width:40rem;margin:3rem auto;padding:1.5rem;font-family:system-ui,sans-serif">
          <h1 style="font-size:1.25rem;margin:0 0 0.75rem">UI failed to start</h1>
          <p>Please refresh the page or check the server logs.</p>
          <pre style="white-space:pre-wrap;color:#b91c1c;background:#fef2f2;padding:1rem;border-radius:8px">${err && err.stack ? err.stack : String(err)}</pre>
        </div>
      `;
    }
  });
