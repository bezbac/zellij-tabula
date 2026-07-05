import { test, expect } from "@microsoft/tui-test";
import {
  writeLine,
  expectViewToContain,
  expectViewNotToContain,
  maybeApprovePermissions,
} from "./test-utils.js";

test.use({ program: { file: "/bin/zsh" } });

const sessionName = `pane-status-session-${Date.now()}`;
const targetDir = `pane-status-dir-${Date.now()}`;

test("prefixes tab titles when a pane is waiting", async ({ terminal }) => {
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

  writeLine(terminal, `mkdir ${targetDir}`);
  writeLine(terminal, `cd ${targetDir}`);
  await expectViewToContain(terminal, `~/${targetDir} $`, 10000);
  await expectViewToContain(terminal, `Zellij (${sessionName})  ~/${targetDir}`);

  writeLine(
    terminal,
    "zellij pipe --name tabula -- \"status '$ZELLIJ_PANE_ID' 'waiting'\"",
  );
  await expectViewToContain(terminal, `Zellij (${sessionName})  ⏳ ~/${targetDir}`);

  writeLine(
    terminal,
    "zellij pipe --name tabula -- \"status '$ZELLIJ_PANE_ID' 'none'\"",
  );
  await expectViewToContain(terminal, `Zellij (${sessionName})  ~/${targetDir}`);
  await expectViewNotToContain(terminal, `Zellij (${sessionName})  ⏳ ~/${targetDir}`);
});
