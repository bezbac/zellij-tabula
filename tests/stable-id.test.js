import { test, expect } from "@microsoft/tui-test";
import {
  writeLine,
  expectViewToContain,
  expectViewNotToContain,
  pressTabModeKey,
  expectTabTitle,
  maybeApprovePermissions,
} from "./test-utils.js";

test.use({ program: { file: "/bin/zsh" } });

const sessionName = "stable-id-session";

test("handles tab renames after closing a tab", async ({ terminal }) => {
  await expect(
    terminal.getByText("Using config /home/alice/.zshrc", { full: true }),
  ).toBeVisible();

  writeLine(terminal, "cd");
  await expect(terminal.getByText("~ $", { strict: false })).toBeVisible();

  writeLine(terminal, "mkdir -p stable-first stable-second stable-third");
  writeLine(terminal, `zellij attach -c ${sessionName}`);

  await expect(terminal.getByText("Pane #1", { full: true })).toBeVisible();
  await maybeApprovePermissions(terminal);
  await expect(
    terminal.getByText("Using config /home/alice/.zshrc", { full: true }),
  ).toBeVisible();
  await expect(terminal.getByText("~ $", { strict: false })).toBeVisible();

  writeLine(terminal, "cd ~/stable-first");
  await expect(
    terminal.getByText("~/stable-first $", { strict: false }),
  ).toBeVisible();
  await expectTabTitle(terminal, "~/stable-first", sessionName, 8000);

  await pressTabModeKey(terminal, "n");
  writeLine(terminal, "cd ~/stable-second");
  await expect(
    terminal.getByText("~/stable-second $", { strict: false }),
  ).toBeVisible();
  await expectViewToContain(
    terminal,
    `Zellij (${sessionName})  ~/stable-first  ~/stable-second`,
    8000,
  );

  await pressTabModeKey(terminal, "1");
  await expect(
    terminal.getByText("~/stable-first $", { strict: false }),
  ).toBeVisible();
  await expectViewToContain(
    terminal,
    `Zellij (${sessionName})  ~/stable-first  ~/stable-second`,
    8000,
  );

  await pressTabModeKey(terminal, "x");
  await expect(
    terminal.getByText("~/stable-second $", { strict: false }),
  ).toBeVisible();
  await expectTabTitle(terminal, "~/stable-second", sessionName, 8000);

  writeLine(terminal, "cd ~/stable-third");
  await expect(
    terminal.getByText("~/stable-third $", { strict: false }),
  ).toBeVisible();
  await expectTabTitle(terminal, "~/stable-third", sessionName, 8000);
  await expectViewNotToContain(
    terminal,
    `Zellij (${sessionName})  ~/stable-second`,
  );
});
