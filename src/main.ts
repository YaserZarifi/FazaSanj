import { mount } from "svelte";
import "@fontsource/vazirmatn/400.css";
import "@fontsource/vazirmatn/500.css";
import "@fontsource/vazirmatn/700.css";
import "./lib/theme";
import App from "./App.svelte";
import { initBackend } from "./lib/api/client";

const target = document.getElementById("app");
if (!target) throw new Error("missing #app");

await initBackend();

export default mount(App, { target });
