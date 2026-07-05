import type { Plugin } from "@opencode-ai/plugin";
import { execFile } from "node:child_process";
import { promisify } from "node:util";

const execFileAsync = promisify(execFile);

const ZellijPlugin: Plugin = async ({ client }) => {
  await client.app.log({
    body: {
      service: "zellij-tabula-opencode-plugin",
      level: "info",
      message: "Zellij-tabula plugin initialized",
    },
  });

  let globalStatus: "waiting" | "none" = "none";

  async function setPaneStatus(status: "waiting" | "none") {
    if (globalStatus === status) {
      return;
    }

    globalStatus = status;

    const paneId = process.env.ZELLIJ_PANE_ID;

    if (paneId === undefined) {
      await client.app.log({
        body: {
          service: "zellij-tabula-opencode-plugin",
          level: "debug",
          message: `ZELLIJ_PANE_ID environment variable is not set. Skipping pane status update.`,
        },
      });

      return;
    }

    await client.app.log({
      body: {
        service: "zellij-tabula-opencode-plugin",
        level: "debug",
        message: `Setting zellij pane status to '${status}' for pane ID '${paneId}'`,
      },
    });

    await execFileAsync("zellij", [
      "pipe",
      "--name",
      "tabula",
      "--",
      `status '${paneId}' '${status}'`,
    ]);
  }

  await setPaneStatus("none");

  return {
    event: async ({ event }) => {
      // The types are wrong here, "permission.asked" isn't listed but does get emitted.
      // See: https://github.com/anomalyco/opencode/issues/7006#issuecomment-4092941620
      if ((event.type as string) === "permission.asked") {
        return setPaneStatus("waiting");
      }

      if (event.type === "permission.replied") {
        return setPaneStatus("none");
      }
    },

    dispose: async () => {
      return setPaneStatus("none");
    },
  };
};

export default ZellijPlugin;
