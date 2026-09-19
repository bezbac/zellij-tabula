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

    await waitFor(20);
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

    await waitFor(20);
    view = terminal.serialize().view;
  }

  throw new Error(`Still found "${unexpectedText}" in:\n${view}`);
}

export async function pressTabModeKey(
  terminal,
  key,
  expectedText,
  timeout = 5000,
) {
  terminal.write("\u0014");
  terminal.write(key);
  if (expectedText) {
    await expectViewToContain(terminal, expectedText, timeout);
  } else {
    // No view marker exists for tab focus, and zellij's tab mode stays
    // active after the action key and would swallow the next keystroke.
    // Give it a beat to return to normal mode before the caller sends input.
    await waitFor(50);
  }
}

// zellij only re-sends TabUpdate on a layout change,
// so a plugin that subscribes after the initial snapshot
// never learns about the first tab and won't rename it.
// Force a fresh TabUpdate by creating and immediately closing a new tab.
export async function waitForPluginLoad(terminal) {
  await pressTabModeKey(terminal, "n", "Tab #2");
  await pressTabModeKey(terminal, "x");
  await expectViewNotToContain(terminal, "Tab #2");
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
