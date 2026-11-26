import { afterEach, expect, test } from "bun:test";
import { cleanup, render, screen } from "@octanejs/testing-library";
import { Greeting } from "./Greeting";

afterEach(cleanup);

test("見出しと本文に props の文言を出す", () => {
  render(Greeting, {
    props: { title: "Pelway", message: "土台が動いています" },
  });
  expect(screen.getByRole("heading", { level: 1 }).textContent).toBe("Pelway");
  expect(screen.getByText("土台が動いています")).toBeTruthy();
});
