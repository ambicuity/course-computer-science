"""main.py — character trie with prefix-iteration in Python."""
from __future__ import annotations
from dataclasses import dataclass, field


@dataclass
class TrieNode:
    children: dict[str, "TrieNode"] = field(default_factory=dict)
    terminal: bool = False


class Trie:
    def __init__(self) -> None: self.root = TrieNode()

    def insert(self, word: str) -> None:
        cur = self.root
        for c in word:
            if c not in cur.children:
                cur.children[c] = TrieNode()
            cur = cur.children[c]
        cur.terminal = True

    def contains(self, word: str) -> bool:
        cur = self.root
        for c in word:
            if c not in cur.children: return False
            cur = cur.children[c]
        return cur.terminal

    def prefix(self, prefix: str) -> list[str]:
        cur = self.root
        for c in prefix:
            if c not in cur.children: return []
            cur = cur.children[c]
        out: list[str] = []
        def go(n: TrieNode, path: str) -> None:
            if n.terminal: out.append(path)
            for ch, child in n.children.items():
                go(child, path + ch)
        go(cur, prefix)
        return out


def main() -> None:
    t = Trie()
    for w in ["cat", "car", "card", "care", "careful", "core", "dog", "dot"]:
        t.insert(w)
    print("contains 'card':", t.contains("card"))
    print("contains 'ca':   ", t.contains("ca"))
    print("prefix 'ca':     ", t.prefix("ca"))
    print("prefix 'd':      ", t.prefix("d"))


if __name__ == "__main__":
    main()
