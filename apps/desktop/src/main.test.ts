import { expect, it, vi } from "vitest";

const mounted = vi.hoisted(() => ({ selector: "", component: undefined as unknown }));

vi.mock("vue", async (importOriginal) => ({
  ...(await importOriginal<typeof import("vue")>()),
  createApp: (component: unknown) => {
    mounted.component = component;
    return { mount: (selector: string) => { mounted.selector = selector; } };
  },
}));
vi.mock("./App.vue", () => ({ default: { name: "App" } }));

it("mounts the App component on #app", async () => {
  await import("./main");
  expect(mounted.selector).toBe("#app");
  expect(mounted.component).toEqual({ name: "App" });
});
