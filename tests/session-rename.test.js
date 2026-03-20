import { test, expect } from "@microsoft/tui-test";
import {
  writeLine,
  expectViewToContain,
  maybeApprovePermissions,
} from "./test-utils.js";

test.use({ program: { file: "/bin/zsh" } });

const initialSessionName = `rename-session-${Date.now()}`;
const renamedSessionName = `${initialSessionName}-renamed`;
const targetDir = `renamed-session-test-${Date.now()}`;
const backgroundSessionName = `${initialSessionName}-background`;

test("continues handling cd after renaming the session with multiple sessions present", async ({ terminal }) => {
  await expect(
    terminal.getByText("Using config /home/alice/.zshrc", { full: true }),
  ).toBeVisible();

  writeLine(terminal, "cd");
  await expect(terminal.getByText("~ $", { strict: false })).toBeVisible();

  writeLine(terminal, `zellij attach -b ${backgroundSessionName}`);
  await expectViewToContain(terminal, "~ $", 10000);

  writeLine(terminal, `zellij attach -c ${initialSessionName}`);
  await expect(terminal.getByText("Pane #1", { full: true })).toBeVisible({
    timeout: 10000,
  });
  await maybeApprovePermissions(terminal);
  await expect(
    terminal.getByText("Using config /home/alice/.zshrc", { full: true }),
  ).toBeVisible();
  await expect(terminal.getByText("~ $", { strict: false })).toBeVisible();
  await expectViewToContain(terminal, `Zellij (${initialSessionName})  Tab #1`);

  writeLine(terminal, `mkdir ${targetDir}`);
  await expectViewToContain(terminal, "~ $", 10000);

  writeLine(terminal, `zellij action rename-session ${renamedSessionName}`);
  writeLine(terminal, "env | sort | grep '^ZELLIJ_SESSION_NAME='");
  await expectViewToContain(
    terminal,
    `ZELLIJ_SESSION_NAME=${renamedSessionName}`,
    10000,
  );

  writeLine(terminal, `cd ${targetDir}`);
  await expectViewToContain(terminal, `~/${targetDir} $`, 10000);

  writeLine(terminal, "pwd");
  await expectViewToContain(terminal, `/home/alice/${targetDir}`, 10000);
});
