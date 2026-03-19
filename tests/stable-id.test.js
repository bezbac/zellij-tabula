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

const sessionName = `stable-id-session-${Date.now()}`;

const secondTabName = "second";
const thirdTabName = "third";
const fourthTabName = "fourth";
const fifthTabName = "fifth";

function tabBarText(tabTitles) {
  return `Zellij (${sessionName})  ${tabTitles.join("  ")}`;
}

async function expectTabBar(terminal, tabTitles, timeout = 8000) {
  await expectViewToContain(terminal, tabBarText(tabTitles), timeout);
}

async function newTab(terminal) {
  await pressTabModeKey(terminal, "n");
}

async function goToTab(terminal, tabNumber) {
  await pressTabModeKey(terminal, String(tabNumber));
}

async function closeFocusedTab(terminal) {
  await pressTabModeKey(terminal, "x");
}

async function cdIntoTabNameDir(terminal, tabName) {
  writeLine(terminal, `cd ~/${tabName}`);
  await expect(
    terminal.getByText(`~/${tabName} $`, { strict: false }),
  ).toBeVisible();
}

async function createTabNameDirs(terminal, tabNames) {
  writeLine(
    terminal,
    `mkdir -p ${tabNames.map((tabName) => `"${tabName}"`).join(" ")}`,
  );
}

test("handles auto tab names after closing tabs", async ({ terminal }) => {
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
  await createTabNameDirs(terminal, [
    secondTabName,
    thirdTabName,
    fourthTabName,
    fifthTabName,
  ]);

  await expectTabTitle(terminal, "Tab #1", sessionName, 8000);

  for (let tabCount = 1; tabCount < 6; tabCount += 1) {
    await newTab(terminal);
  }

  await expectViewToContain(terminal, "← +3  Tab #4  Tab #5  Tab #6", 8000);

  // Name tab #2 by entering ~/second.
  await goToTab(terminal, 2);
  await cdIntoTabNameDir(terminal, secondTabName);
  await expectViewToContain(
    terminal,
    `Tab #1  ~/${secondTabName}  Tab #3`,
    8000,
  );
  await expectViewToContain(terminal, "+3", 8000);

  // Delete the visible third tab twice.
  await goToTab(terminal, 3);
  await closeFocusedTab(terminal);
  await expectViewToContain(
    terminal,
    `Tab #1  ~/${secondTabName}  Tab #4`,
    8000,
  );
  await expectViewToContain(terminal, "+2", 8000);
  await expectViewNotToContain(terminal, "Tab #3");

  await goToTab(terminal, 3);
  await closeFocusedTab(terminal);
  await expectTabBar(terminal, [
    "Tab #1",
    `~/${secondTabName}`,
    "Tab #5",
    "Tab #6",
  ]);
  await expectViewNotToContain(terminal, "Tab #4");

  // The old fifth tab is now visible tab #3, so entering ~/third should name it.
  await goToTab(terminal, 3);
  await cdIntoTabNameDir(terminal, thirdTabName);
  await expectTabBar(terminal, [
    "Tab #1",
    `~/${secondTabName}`,
    `~/${thirdTabName}`,
    "Tab #6",
  ]);

  // The old sixth tab is now visible tab #4, so entering ~/fourth should name it.
  await goToTab(terminal, 4);
  await cdIntoTabNameDir(terminal, fourthTabName);
  await expectTabBar(terminal, [
    "Tab #1",
    `~/${secondTabName}`,
    `~/${thirdTabName}`,
    `~/${fourthTabName}`,
  ]);
  await expectViewNotToContain(terminal, "Tab #5");
  await expectViewNotToContain(terminal, "Tab #6");

  // Renaming the surviving fourth visible tab again should not affect tab #3.
  await goToTab(terminal, 4);
  await cdIntoTabNameDir(terminal, fifthTabName);
  await expectTabBar(terminal, [
    "Tab #1",
    `~/${secondTabName}`,
    `~/${thirdTabName}`,
    `~/${fifthTabName}`,
  ]);
});
