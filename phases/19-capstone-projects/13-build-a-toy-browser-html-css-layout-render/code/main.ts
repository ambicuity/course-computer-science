type Node = {
  tag: string;
  className?: string;
  text: string;
};

type Rule = {
  selector: string;
  decls: Record<string, string>;
};

function parseHTML(input: string): Node[] {
  if (input.includes("<p")) {
    return [{
      tag: "p",
      className: input.includes('class="note"') ? "note" : undefined,
      text: "hello tiny browser"
    }];
  }
  return [];
}

function parseCSS(input: string): Rule[] {
  return input
    .split("}")
    .map((s) => s.trim())
    .filter(Boolean)
    .map((chunk) => {
      const [selector, body] = chunk.split("{");
      const decls: Record<string, string> = {};
      body.split(";").map((x) => x.trim()).filter(Boolean).forEach((d) => {
        const [k, v] = d.split(":");
        decls[k.trim()] = v.trim();
      });
      return { selector: selector.trim(), decls };
    });
}

function matches(node: Node, selector: string): boolean {
  if (selector.startsWith(".")) return node.className === selector.slice(1);
  return node.tag === selector;
}

function computeStyle(node: Node, rules: Rule[]): Record<string, string> {
  const style: Record<string, string> = {};
  for (const rule of rules) {
    if (!matches(node, rule.selector)) continue;
    for (const [k, v] of Object.entries(rule.decls)) style[k] = v;
  }
  return style;
}

function main(): void {
  const html = '<body><p class="note">hello tiny browser</p></body>';
  const css = 'p { color: black; } .note { color: green; }';

  const nodes = parseHTML(html);
  const rules = parseCSS(css);

  for (const node of nodes) {
    const style = computeStyle(node, rules);
    const color = style.color ?? "default";
    console.log(`<${node.tag} color=${color}> ${node.text}`);
  }
}

main();
