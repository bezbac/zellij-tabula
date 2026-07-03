import { test, expect } from "@microsoft/tui-test";
import {
  expectViewToContain,
  expectViewNotToContain,
  maybeApprovePermissions,
} from "./test-utils.js";

test.use({ program: { file: "/bin/zsh" } });

const sessionName = "example-session";

test("renames tab on navigation", async ({ terminal }) => {
  await expect(
    terminal.getByText("Using config /home/alice/.zshrc", { full: true }),
  ).toBeVisible();

  terminal.write("whoami");
  terminal.write("\r");

  await expect(terminal.getByText("alice", { full: true })).toBeVisible();

  terminal.write("echo $HOME");
  terminal.write("\r");

  await expect(terminal.getByText("/home/alice", { full: true })).toBeVisible();

  terminal.write("cd");
  terminal.write("\r");

  terminal.write("mkdir test1");
  terminal.write("\r");

  terminal.write("mkdir test2");
  terminal.write("\r");

  terminal.write("ls");
  terminal.write("\r");

  terminal.write(`zellij attach -c ${sessionName}`);
  terminal.write("\r");

  await expect(terminal.getByText("Pane #1", { full: true })).toBeVisible();
  await expect(
    terminal.getByText(`Zellij (${sessionName})  Tab #1`, { full: true }),
  ).toBeVisible();

  await maybeApprovePermissions(terminal);

  await expect(
    terminal.getByText("Using config /home/alice/.zshrc", { full: true }),
  ).toBeVisible();

  await expect(terminal.getByText("~ $", { full: true })).toBeVisible();

  terminal.write("cd test1");
  terminal.write("\r");

  await expect(terminal.getByText("~/test1 $", { full: true })).toBeVisible();

  await expect(terminal.getByText("Pane #1", { full: true })).toBeVisible();
  await expect(
    terminal.getByText(`Zellij (${sessionName})  Tab #1`, { full: true }),
  ).not.toBeVisible();
  await expect(
    terminal.getByText(`Zellij (${sessionName})  ~/test1`, { full: true }),
  ).toBeVisible();

  terminal.write("cd ~");
  terminal.write("\r");

  await expect(terminal.getByText("Pane #1", { full: true })).toBeVisible();
  await expect(
    terminal.getByText(`Zellij (${sessionName})  ~/test1`, { full: true }),
  ).not.toBeVisible();
  await expect(
    terminal.getByText(`Zellij (${sessionName})  ~`, { full: true }),
  ).toBeVisible();

  terminal.write("mkdir -p git-project/src/nested");
  terminal.write("\r");
  terminal.write("cd git-project");
  terminal.write("\r");
  terminal.write("git init -q");
  terminal.write("\r");
  terminal.write("cd src/nested");
  terminal.write("\r");

  await expect(
    terminal.getByText("~/git-project/src/nested $", { full: true }),
  ).toBeVisible();

  await expectViewToContain(
    terminal,
    `Zellij (${sessionName})  git-project/src/nested`,
  );
  await expectViewNotToContain(
    terminal,
    `Zellij (${sessionName})  ~/git-project/src/nested`,
  );

  terminal.write("cd ../..");
  terminal.write("\r");

  await expectViewToContain(terminal, `Zellij (${sessionName})  git-project`);

  terminal.write("git config user.email test@example.com");
  terminal.write("\r");
  terminal.write("git config user.name 'Tabula Test'");
  terminal.write("\r");
  terminal.write("touch src/nested/.keep");
  terminal.write("\r");
  terminal.write("git add .");
  terminal.write("\r");
  terminal.write("git commit -qm init");
  terminal.write("\r");
  terminal.write("git worktree add -q ../git-project-worktree -b worktree-test");
  terminal.write("\r");
  terminal.write("cd ../git-project-worktree/src/nested");
  terminal.write("\r");

  await expect(
    terminal.getByText("~/git-project-worktree/src/nested $", { full: true }),
  ).toBeVisible();

  await expectViewToContain(
    terminal,
    `Zellij (${sessionName})  git-project/src/nested (🌲 git-projec...)`,
  );
  await expectViewNotToContain(
    terminal,
    `Zellij (${sessionName})  git-project-worktree/src/nested`,
  );

  terminal.write("cd ../..");
  terminal.write("\r");

  await expectViewToContain(
    terminal,
    `Zellij (${sessionName})  git-project (🌲 git-projec...)`,
  );
});
