import * as vscode from "vscode";
import * as fs from "fs";
import * as path from "path";
import { readEntries } from "./ledgerProvider";
import { newUlid, hashInput, appendEntry, LedgerError } from "./ledgerWriter";

export function registerChatParticipant(
  context: vscode.ExtensionContext,
  harnessRoot: string,
  sessionsDir: string,
): vscode.Disposable {
  const participant = vscode.chat.createChatParticipant(
    "harness",
    async (
      request: vscode.ChatRequest,
      chatContext: vscode.ChatContext,
      stream: vscode.ChatResponseStream,
      token: vscode.CancellationToken,
    ) => {
      if (request.command === "ledger") {
        return showLedgerSummary(stream, sessionsDir);
      }

      const [model] = await vscode.lm.selectChatModels({
        vendor: request.model?.vendor,
        family: request.model?.family,
        id: request.model?.id,
      });

      if (!model) {
        stream.markdown("**Harness:** No language model available. Make sure GitHub Copilot is signed in.");
        return;
      }

      const lmMessages: vscode.LanguageModelChatMessage[] = [];
      for (const turn of chatContext.history) {
        if (turn instanceof vscode.ChatRequestTurn) {
          lmMessages.push(vscode.LanguageModelChatMessage.User(turn.prompt));
        } else if (turn instanceof vscode.ChatResponseTurn) {
          const text = turn.response
            .map((r) => r instanceof vscode.ChatResponseMarkdownPart ? r.value.value : "")
            .join("");
          if (text) lmMessages.push(vscode.LanguageModelChatMessage.Assistant(text));
        }
      }

      // Resolve #file / #selection / #codebase references attached to this
      // request and prepend their content so the model has full workspace
      // context — without this the model is blind to any referenced files.
      const refParts: vscode.LanguageModelTextPart[] = [];
      for (const ref of request.references) {
        try {
          if (ref.value instanceof vscode.Uri) {
            const bytes = await vscode.workspace.fs.readFile(ref.value);
            const text = Buffer.from(bytes).toString("utf8");
            refParts.push(
              new vscode.LanguageModelTextPart(
                `### File: ${ref.value.fsPath}\n\`\`\`\n${text}\n\`\`\`\n`,
              ),
            );
          } else if (ref.value instanceof vscode.Location) {
            const doc = await vscode.workspace.openTextDocument(ref.value.uri);
            const text = doc.getText(ref.value.range);
            refParts.push(
              new vscode.LanguageModelTextPart(
                `### Selection from: ${ref.value.uri.fsPath}\n\`\`\`\n${text}\n\`\`\`\n`,
              ),
            );
          }
        } catch {
          // If a reference can't be resolved, skip it — don't abort the request.
        }
      }

      // Build the final user message: references block (if any) + prompt.
      const userContent: (vscode.LanguageModelTextPart)[] = [
        ...(refParts.length ? [new vscode.LanguageModelTextPart("The following files/selections were attached to this request:\n\n"), ...refParts] : []),
        new vscode.LanguageModelTextPart(request.prompt),
      ];
      lmMessages.push(new vscode.LanguageModelChatMessage(vscode.LanguageModelChatMessageRole.User, userContent));

      const plainMessages = lmMessages.map((m) => ({
        role: m.role === vscode.LanguageModelChatMessageRole.User ? "user" : "assistant",
        content: (Array.isArray(m.content)
          ? (m.content as vscode.LanguageModelTextPart[])
              .filter((p) => typeof p.value === "string")
              .map((p) => p.value)
              .join("")
          : String(m.content)),
      }));
      const inHash = hashInput(null, plainMessages);
      const sid = newUlid();

      stream.progress(`Thinking via ${model.name} (harnessed)�`);

      let fullResponse = "";
      try {
        const response = await model.sendRequest(lmMessages, {}, token);
        for await (const chunk of response.text) {
          fullResponse += chunk;
          if (token.isCancellationRequested) break;
        }
      } catch (e: unknown) {
        stream.markdown(`**Model error:** ${e instanceof Error ? e.message : String(e)}`);
        return;
      }

      let entry;
      try {
        entry = appendEntry(harnessRoot, sid, model.id, inHash, fullResponse, null);
      } catch (e) {
        if (e instanceof LedgerError) {
          stream.markdown(`**Harness FAIL-CLOSED:** Ledger write failed � response withheld.\n\n\`${e.message}\``);
          return;
        }
        throw e;
      }

      stream.markdown(fullResponse);
      stream.markdown(
        `\n\n---\n*Harnessed � model \`${model.id}\` � session \`${entry.sid}\` � entry #${entry.seq} � prev \`${entry.prev.slice(0, 16)}�\`*`
      );
    },
  );

  participant.iconPath = new vscode.ThemeIcon("law");
  return { dispose() { participant.dispose(); } };
}

async function showLedgerSummary(stream: vscode.ChatResponseStream, sessionsDir: string): Promise<void> {
  if (!fs.existsSync(sessionsDir)) {
    stream.markdown("No sessions on disk yet.");
    return;
  }
  const files = fs.readdirSync(sessionsDir).filter((f) => f.endsWith(".jsonl")).sort().reverse().slice(0, 10);
  if (!files.length) { stream.markdown("No sessions found."); return; }
  const lines = ["### Harness Ledger � last 10 sessions", ""];
  for (const f of files) {
    const entries = readEntries(path.join(sessionsDir, f));
    const last = entries.at(-1);
    lines.push(`- **\`${f.replace(".jsonl", "")}\`** � ${entries.length} entries � last model: \`${last?.model ?? "?"}\``);
  }
  stream.markdown(lines.join("\n"));
}
