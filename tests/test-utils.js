import { expect } from "@microsoft/tui-test";

export function waitFor(ms) {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

export function writeLine(terminal, value) {
  terminal.write(value);
  terminal.write("\r");
}

export async function expectViewToContain(
  terminal,
  expectedText,
  timeout = 5000,
) {
  const deadline = Date.now() + timeout;
  let view = terminal.serialize().view;

  while (Date.now() < deadline) {
    if (view.includes(expectedText)) {
      return;
    }

    await waitFor(100);
    view = terminal.serialize().view;
  }

  throw new Error(`Timed out waiting for "${expectedText}" in:\n${view}`);
}

export async function expectViewNotToContain(
  terminal,
  unexpectedText,
  timeout = 2000,
) {
  const deadline = Date.now() + timeout;
  let view = terminal.serialize().view;

  while (Date.now() < deadline) {
    if (!view.includes(unexpectedText)) {
      return;
    }

    await waitFor(100);
    view = terminal.serialize().view;
  }

  throw new Error(`Still found "${unexpectedText}" in:\n${view}`);
}

export async function pressTabModeKey(terminal, key) {
  terminal.write("\u0014");
  await waitFor(100);
  terminal.write(key);
  await waitFor(1000);
}

export async function expectTabTitle(
  terminal,
  expectedTabTitle,
  sessionName,
  timeout = 5000,
) {
  await expectViewToContain(
    terminal,
    `Zellij (${sessionName})  ${expectedTabTitle}`,
    timeout,
  );
}

export async function maybeApprovePermissions(terminal) {
  const permissionPrompt = terminal.getByText(
    "Plugin /zellij-tabula.wasm asks",
    {
      full: true,
    },
  );

  try {
    await expect(permissionPrompt).toBeVisible({ timeout: 10000 });
    terminal.write("y");
    await expect(permissionPrompt).not.toBeVisible({ timeout: 5000 });
  } catch {
    // Zellij may persist the plugin permission decision between runs.
  }
}
