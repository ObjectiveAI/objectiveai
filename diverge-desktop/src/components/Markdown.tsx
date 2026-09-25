// Just enough Markdown for what agents write: paragraphs, lists, bold,
// inline code and fenced code. Nothing is ever rendered as HTML.

import type { ReactNode } from "react";

function inline(text: string): ReactNode[] {
  const out: ReactNode[] = [];
  const pattern = /(\*\*[^*]+\*\*|`[^`]+`)/g;
  let last = 0;
  let m: RegExpExecArray | null;
  let i = 0;
  while ((m = pattern.exec(text))) {
    if (m.index > last) out.push(text.slice(last, m.index));
    const token = m[0];
    out.push(token.startsWith("**") ? <strong key={i++}>{token.slice(2, -2)}</strong> : <code key={i++}>{token.slice(1, -1)}</code>);
    last = m.index + token.length;
  }
  if (last < text.length) out.push(text.slice(last));
  return out;
}

export function Markdown({ text }: { text: string }) {
  const lines = text.split("\n");
  const nodes: ReactNode[] = [];
  let list: { ordered: boolean; items: string[] } | null = null;
  let para: string[] = [];
  let code: string[] | null = null;
  let k = 0;
  const flushPara = () => {
    if (para.length) nodes.push(<p key={k++}>{inline(para.join(" "))}</p>);
    para = [];
  };
  const flushList = () => {
    if (list) {
      const items = list.items.map((item, i) => <li key={i}>{inline(item)}</li>);
      nodes.push(list.ordered ? <ol key={k++}>{items}</ol> : <ul key={k++}>{items}</ul>);
    }
    list = null;
  };
  for (const line of lines) {
    if (code) {
      if (line.startsWith("```")) {
        nodes.push(<pre key={k++}>{code.join("\n")}</pre>);
        code = null;
      } else code.push(line);
      continue;
    }
    if (line.startsWith("```")) {
      flushPara();
      flushList();
      code = [];
      continue;
    }
    const bullet = /^\s*[-*]\s+(.*)$/.exec(line);
    const numbered = /^\s*\d+\.\s+(.*)$/.exec(line);
    if (bullet || numbered) {
      flushPara();
      const ordered = !!numbered;
      if (!list || list.ordered !== ordered) {
        flushList();
        list = { ordered, items: [] };
      }
      list.items.push((bullet ?? numbered)![1]);
      continue;
    }
    if (!line.trim()) {
      flushPara();
      flushList();
      continue;
    }
    flushList();
    para.push(line);
  }
  if (code) nodes.push(<pre key={k++}>{(code as string[]).join("\n")}</pre>);
  flushPara();
  flushList();
  return <div className="md selectable">{nodes}</div>;
}
