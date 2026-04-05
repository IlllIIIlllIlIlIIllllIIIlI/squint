import * as fs from "fs";
import * as path from "path";
import { ExtensionContext, window, workspace } from "vscode";
import {
  LanguageClient,
  LanguageClientOptions,
  ServerOptions,
  TransportKind,
} from "vscode-languageclient/node";

let client: LanguageClient | undefined;

export function activate(context: ExtensionContext): void {
  const serverPath = resolveServerPath();
  if (!serverPath) {
    window.showErrorMessage(
      "squint: could not find squint-lsp binary. " +
        "Install it with `cargo install squint --features lsp --bin squint-lsp` " +
        "or set `squint.serverPath` in your VS Code settings."
    );
    return;
  }

  const serverOptions: ServerOptions = {
    command: serverPath,
    args: [],
    transport: TransportKind.stdio,
  };

  const clientOptions: LanguageClientOptions = {
    documentSelector: [
      { scheme: "file", language: "sql" },
      { scheme: "file", language: "jinja-sql" },
    ],
    synchronize: {
      fileEvents: workspace.createFileSystemWatcher(
        "**/{squint.toml,pyproject.toml}"
      ),
    },
  };

  client = new LanguageClient(
    "squint",
    "squint SQL linter",
    serverOptions,
    clientOptions
  );

  client.start();
  context.subscriptions.push(client);
}

export function deactivate(): Thenable<void> | undefined {
  if (!client) {
    return undefined;
  }
  return client.stop();
}

function resolveServerPath(): string | undefined {
  const binaryName =
    process.platform === "win32" ? "squint-lsp.exe" : "squint-lsp";

  // 1. Honour explicit user setting
  const config = workspace.getConfiguration("squint");
  const configuredPath = config.get<string>("serverPath");
  if (configuredPath && configuredPath.trim() !== "") {
    const expanded = configuredPath.replace(
      /^~/,
      process.env.HOME ?? process.env.USERPROFILE ?? ""
    );
    if (fs.existsSync(expanded)) {
      return expanded;
    }
    window.showWarningMessage(
      `squint: configured serverPath "${expanded}" not found, falling back to PATH.`
    );
  }

  // 2. Search PATH manually — VS Code on macOS/Linux may launch with a
  //    restricted PATH that doesn't include ~/.cargo/bin or shell profile paths.
  const pathEnv = process.env.PATH ?? "";
  for (const dir of pathEnv.split(path.delimiter)) {
    const candidate = path.join(dir, binaryName);
    if (fs.existsSync(candidate)) {
      return candidate;
    }
  }

  // 3. Common cargo install location as a last resort
  const homeDir = process.env.HOME ?? process.env.USERPROFILE ?? "";
  const cargoCandidate = path.join(homeDir, ".cargo", "bin", binaryName);
  if (fs.existsSync(cargoCandidate)) {
    return cargoCandidate;
  }

  return undefined;
}
