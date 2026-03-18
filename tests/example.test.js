import { test, expect } from "@microsoft/tui-test";

test.use({ program: { file: "/bin/zsh" } });

function waitFor(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

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

  terminal.write("zellij attach -c session");
  terminal.write("\r");

  await expect(terminal.getByText("Pane #1", { full: true })).toBeVisible();
  await expect(
    terminal.getByText("Zellij (session)  Tab #1", { full: true }),
  ).toBeVisible();

  await expect(
    terminal.getByText("Plugin /zellij-tabula.wasm asks", { full: true }),
  ).toBeVisible();

  terminal.write("y");

  await expect(
    terminal.getByText("Plugin /zellij-tabula.wasm asks", { full: true }),
  ).not.toBeVisible();

  await expect(
    terminal.getByText("Using config /home/alice/.zshrc", { full: true }),
  ).toBeVisible();

  await expect(terminal.getByText("~ $", { full: true })).toBeVisible();

  terminal.write("cd test1");
  terminal.write("\r");

  await expect(terminal.getByText("~/test1 $", { full: true })).toBeVisible();

  await expect(terminal.getByText("Pane #1", { full: true })).toBeVisible();
  await expect(
    terminal.getByText("Zellij (session)  Tab #1", { full: true }),
  ).not.toBeVisible();
  await expect(
    terminal.getByText("Zellij (session)  ~/test1", { full: true }),
  ).toBeVisible();

  terminal.write("cd ~");
  terminal.write("\r");

  await waitFor(1000);

  await expect(terminal.getByText("Pane #1", { full: true })).toBeVisible();
  await expect(
    terminal.getByText("Zellij (session)  ~/test1", { full: true }),
  ).not.toBeVisible();
  await expect(
    terminal.getByText("Zellij (session)  ~", { full: true }),
  ).toBeVisible();

  terminal.write(
    "mkdir -p git-project/src/nested && cd git-project && git init -q && cd src/nested",
  );
  terminal.write("\r");

  await expect(
    terminal.getByText("~/git-project/src/nested $", { full: true }),
  ).toBeVisible();

  await expect(
    terminal.getByText("Zellij (session)  git-project/src/nested", {
      full: true,
    }),
  ).toBeVisible();
  await expect(
    terminal.getByText("Zellij (session)  ~/git-project/src/nested", {
      full: true,
    }),
  ).not.toBeVisible();

  terminal.write("cd ../..");
  terminal.write("\r");

  await expect(
    terminal.getByText("Zellij (session)  git-project", { full: true }),
  ).toBeVisible();
});
