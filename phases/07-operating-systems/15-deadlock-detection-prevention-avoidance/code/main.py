from __future__ import annotations

from typing import Dict, List


def has_cycle(wait_for: Dict[str, List[str]]) -> bool:
    seen: set[str] = set()
    stack: set[str] = set()

    def dfs(n: str) -> bool:
        if n in stack:
            return True
        if n in seen:
            return False
        seen.add(n)
        stack.add(n)
        for m in wait_for.get(n, []):
            if dfs(m):
                return True
        stack.remove(n)
        return False

    return any(dfs(n) for n in wait_for)


def main() -> None:
    graph = {"P1": ["P2"], "P2": ["P3"], "P3": ["P1"]}
    print(f"deadlock_detected={has_cycle(graph)}")


if __name__ == "__main__":
    main()
