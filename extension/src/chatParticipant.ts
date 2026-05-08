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
      try {
        return await handleRequest(request, chatContext, stream, token, harnessRoot, sessionsDir);
      } catch (e: unknown) {
        // Top-level guard: surface every unexpected error in the chat so the
        // participant does NOT go silent. Without this VS Code kills the
        // participant after any unhandled rejection and subsequent messages
        // get no response.
        stream.markdown(`**Harness internal error** (participant kept alive):\n\n\`\`\`\n${e instanceof Error ? e.stack ?? e.message : String(e)}\n\`\`\``);
      }
    },
  );

  participant.iconPath = new vscode.ThemeIcon("law");
  return { dispose() { participant.dispose(); } };
}

async function handleRequest(
  request: vscode.ChatRequest,
  chatContext: vscode.ChatContext,
  stream: vscode.ChatResponseStream,
  token: vscode.CancellationToken,
  harnessRoot: string,
  sessionsDir: string,
): Promise<void> {
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
      let sid = "";
      // Footer regex — must match the 26-char ULID we now generate correctly.
      // Very permissive on surrounding whitespace so VS Code markdown normalization
      // doesn't break it.
      const footerRegex = /\*Harnessed[^*]*session\s*`([A-Z0-9]{26})`[^*]*\*/;

      for (const turn of chatContext.history) {
        if (turn instanceof vscode.ChatRequestTurn) {
          lmMessages.push(vscode.LanguageModelChatMessage.User(turn.prompt));
        } else if (turn instanceof vscode.ChatResponseTurn) {
          let text = turn.response
            .map((r) => r instanceof vscode.ChatResponseMarkdownPart ? r.value.value : "")
            .join("");

          if (text) {
            const match = text.match(footerRegex);
            if (match) {
              sid = match[1];
              // Strip the footer and the preceding HR so the model doesn't see it
              text = text.replace(/\n*---\n*\*Harnessed[^*]*\*/, "").trimEnd();
            }
            lmMessages.push(vscode.LanguageModelChatMessage.Assistant(text));
          }
        }
      }

      if (!sid) {
        sid = newUlid();
      }

      // Resolve #file / #selection / #codebase references attached to this
      // request and prepend their content so the model has full workspace
      // context — without this the model is blind to any referenced files.
      const refParts: vscode.LanguageModelTextPart[] = [];
      const resolvedRefs: { label: string; uri: vscode.Uri }[] = [];
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
            resolvedRefs.push({ label: ref.value.fsPath, uri: ref.value });
          } else if (ref.value instanceof vscode.Location) {
            const doc = await vscode.workspace.openTextDocument(ref.value.uri);
            const text = doc.getText(ref.value.range);
            refParts.push(
              new vscode.LanguageModelTextPart(
                `### Selection from: ${ref.value.uri.fsPath}\n\`\`\`\n${text}\n\`\`\`\n`,
              ),
            );
            resolvedRefs.push({ label: `${ref.value.uri.fsPath} (selection)`, uri: ref.value.uri });
          } else if (typeof ref.value === "string") {
            refParts.push(new vscode.LanguageModelTextPart(`### Reference: ${ref.id}\n${ref.value}\n`));
          }
        } catch (e) {
          stream.markdown(`*Harness: failed to resolve reference \`${ref.id}\`: ${e instanceof Error ? e.message : String(e)}*\n\n`);
        }
      }

      // Fallback: if no explicit references, include the active editor's
      // visible content so the model has *something* to look at.
      const activeEditor = vscode.window.activeTextEditor;
      if (!refParts.length && activeEditor) {
        const text = activeEditor.document.getText();
        refParts.push(
          new vscode.LanguageModelTextPart(
            `### Active editor: ${activeEditor.document.uri.fsPath}\n\`\`\`\n${text}\n\`\`\`\n`,
          ),
        );
        resolvedRefs.push({ label: `${activeEditor.document.uri.fsPath} (active editor, no refs attached)`, uri: activeEditor.document.uri });
      }

      // Surface what we forwarded so the user can see context the model received.
      for (const r of resolvedRefs) {
        try { stream.reference(r.uri); } catch { /* API may not be available in all VS Code versions */ }
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

      const refSummary = resolvedRefs.length
        ? `${resolvedRefs.length} ref${resolvedRefs.length === 1 ? "" : "s"} attached`
        : `no refs attached`;
      stream.progress(`Thinking via ${model.name} (harnessed, ${refSummary})…`);

      // Pass all available VS Code LM tools so the model can actually invoke
      // file reads, searches, terminal commands etc — identical behaviour to
      // native Copilot Chat, just with every exchange ledgered.
      const tools = [...vscode.lm.tools] as vscode.LanguageModelChatTool[];

      let fullResponse = "";
      try {
        // Agentic tool-call loop: keep sending until the model produces a
        // pure-text response with no pending tool calls.
        while (!token.isCancellationRequested) {
          const response = await model.sendRequest(
            lmMessages,
            tools.length ? { tools } : {},
            token,
          );

          const toolCalls: vscode.LanguageModelToolCallPart[] = [];
          let textThisTurn = "";

          for await (const chunk of response.stream) {
            if (token.isCancellationRequested) break;
            if (chunk instanceof vscode.LanguageModelTextPart) {
              textThisTurn += chunk.value;
              stream.markdown(chunk.value);
            } else if (chunk instanceof vscode.LanguageModelToolCallPart) {
              toolCalls.push(chunk);
            }
          }

          fullResponse += textThisTurn;

          if (toolCalls.length === 0) {
            break; // done — pure text response
          }

          // Add assistant turn with tool calls, then invoke each tool and
          // feed results back before the next iteration.
          lmMessages.push(new vscode.LanguageModelChatMessage(
            vscode.LanguageModelChatMessageRole.Assistant,
            toolCalls as unknown as vscode.LanguageModelTextPart[],
          ));

          const toolResults: vscode.LanguageModelToolResultPart[] = [];
          for (const tc of toolCalls) {
            try {
              const result = await vscode.lm.invokeTool(
                tc.name,
                { input: tc.input as Record<string, unknown>, toolInvocationToken: request.toolInvocationToken },
                token,
              );
              toolResults.push(new vscode.LanguageModelToolResultPart(tc.callId, result.content as vscode.LanguageModelTextPart[]));
            } catch (toolErr) {
              toolResults.push(new vscode.LanguageModelToolResultPart(
                tc.callId,
                [new vscode.LanguageModelTextPart(`Tool error: ${toolErr instanceof Error ? toolErr.message : String(toolErr)}`)],
              ));
            }
          }

          lmMessages.push(new vscode.LanguageModelChatMessage(
            vscode.LanguageModelChatMessageRole.User,
            toolResults as unknown as vscode.LanguageModelTextPart[],
          ));
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
          stream.markdown(`**Harness FAIL-CLOSED:** Ledger write failed — response withheld.\n\n\`${e.message}\``);
          return;
        }
        // Non-LedgerError — surface it rather than re-throwing (would kill participant).
        stream.markdown(`**Harness ledger error:** ${e instanceof Error ? e.message : String(e)}\n\n*Response was delivered but not ledgered.*`);
        return;
      }

      stream.markdown(
        `\n\n---\n*Harnessed \uD83D\uDCCB model \`${model.id}\` \u00B7 session \`${entry.sid}\` \u00B7 entry #${entry.seq} \u00B7 prev \`${entry.prev.slice(0, 16)}\u2026\`*`
      );
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
