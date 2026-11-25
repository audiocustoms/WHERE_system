// SPA mode: there is no server-side runtime. The Axum backend serves the API and
// the static files; the Tauri app bundles the same output.
export const ssr = false;
export const prerender = false;
