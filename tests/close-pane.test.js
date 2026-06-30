import { test, expect } from "@microsoft/tui-test";
import {
  writeLine,
  waitFor,
  expectViewToContain,
  expectViewNotToContain,
  maybeApprovePermissions,
} from "./test-utils.js";

test.use({ program: { file: "/bin/zsh" } });

const sessionName = `close-pane-session-${Date.now()}`;

test("renames tab when closing a pane", async ({ terminal }) => {
  await expect(
    terminal.getByText("Using config /home/alice/.zshrc", { full: true }),
  ).toBeVisible();

  writeLine(terminal, "cd");
  await expect(terminal.getByText("~ $", { strict: false })).toBeVisible();

  writeLine(terminal, `zellij attach -c ${sessionName}`);
  await expect(terminal.getByText("Pane #1", { full: true })).toBeVisible({
    timeout: 10000,
  });
  await maybeApprovePermissions(terminal);
  await expect(
    terminal.getByText("Using config /home/alice/.zshrc", { full: true }),
  ).toBeVisible();
  await expect(terminal.getByText("~ $", { strict: false })).toBeVisible();

  // Create shared directory structure.
  writeLine(terminal, "mkdir -p shared/abc shared/xyz");
  await expect(terminal.getByText("~ $", { strict: false })).toBeVisible();

  // Navigate pane 1 to shared/abc — tab should be named ~/shared/abc.
  writeLine(terminal, "cd shared/abc");
  await expect(
    terminal.getByText("~/shared/abc $", { strict: false }),
  ).toBeVisible();
  await expectViewToContain(terminal, `Zellij (${sessionName})  ~/shared/abc`);

  // Create a second pane.
  writeLine(terminal, "zellij action new-pane");
  await waitFor(2000);
  await expect(terminal.getByText("~ $", { strict: false })).toBeVisible();

  // Navigate pane 2 to shared/xyz — now both working dirs are known and
  // the tab should show the combined name.
  writeLine(terminal, "cd ~/shared/xyz");
  await expect(
    terminal.getByText("~/shared/xyz $", { strict: false }),
  ).toBeVisible();
  await expectViewToContain(
    terminal,
    `Zellij (${sessionName})  ~/shared/* (2 panes)`,
  );

  // Close pane 2 by exiting its shell.
  writeLine(terminal, "exit");
  await waitFor(2000);
  await expect(
    terminal.getByText("~/shared/abc $", { strict: false }),
  ).toBeVisible();
  await expectViewToContain(terminal, `Zellij (${sessionName})  ~/shared/abc`);
  await expectViewNotToContain(
    terminal,
    `Zellij (${sessionName})  ~/shared/* (2 panes)`,
  );
});
