import json, glob, os
path = r'c:\Users\admin\AppData\Roaming\Code\User\workspaceStorage\e0f7bf4bd6968a2b92372f3f220745a5\GitHub.copilot-chat\transcripts\*.jsonl'
latest_file = max(glob.glob(path), key=os.path.getmtime)
md_path = r'c:\git\harness-protocol\.trail\session\transcript.md'
out = []
with open(latest_file, 'r', encoding='utf-8') as f:
    for line in f:
        line = line.strip()
        if not line: continue
        try:
            obj = json.loads(line)
            record_type = obj.get("type", "")
            data_obj = obj.get("data", {})
            if record_type == "user.message":
                text = data_obj.get("message", "")
                if text: out.append(f"\n**USER:**\n{text}\n")
            elif record_type == "assistant.message":
                msg = data_obj.get("message", "")
                content = data_obj.get("content", "")
                reasoning = data_obj.get("reasoningText", "")
                printed_header = False
                if reasoning:
                    if not printed_header:
                        out.append("\n**ASSISTANT:**\n")
                        printed_header = True
                    out.append(f"<thought>\n{reasoning}\n</thought>\n")
                if msg or content:
                    if not printed_header:
                        out.append("\n**ASSISTANT:**\n")
                        printed_header = True
                    out.append(f"{msg if msg else content}\n")
        except Exception: pass
with open(md_path, 'w', encoding='utf-8') as f:
    f.writelines(out)
