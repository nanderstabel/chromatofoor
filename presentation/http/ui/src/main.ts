import { mount } from "svelte";
import { WorkbenchApp } from "@chromatofoor/ui";

const app = mount(WorkbenchApp, {
  target: document.getElementById("app")!,
});

export default app;
