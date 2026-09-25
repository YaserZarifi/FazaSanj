import { mount } from "svelte";
import "@fontsource/vazirmatn/400.css";
import "@fontsource/vazirmatn/500.css";
import "@fontsource/vazirmatn/700.css";
import App from "./App.svelte";

const target = document.getElementById("app");
if (!target) throw new Error("missing #app");

export default mount(App, { target });
