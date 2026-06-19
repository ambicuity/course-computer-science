# Editor Setup — Neovim/VS Code with LSP, DAP

> An IDE isn't a thing. It's a few protocols (LSP, DAP) and a competent editor that speaks them.

**Type:** Learn
**Languages:** Shell
**Prerequisites:** Phase 00, Lessons 01, 04–07
**Time:** ~60 minutes

## Learning Objectives

- Explain what LSP (Language Server Protocol) and DAP (Debug Adapter Protocol) are, and why both exist as standalone specs.
- Configure either Neovim or VS Code as a real IDE for C, Rust, and Python with go-to-definition, refactors, inline diagnostics, and step-debugging.
- Diagnose a "LSP isn't working" failure by reading the language-server log and the editor's LSP client log.
- Apply the same configuration to a fresh machine in under 10 minutes.

## The Problem

Years ago "IDE" meant a giant tool (Eclipse, Visual Studio, JetBrains) that re-implemented every language's parser, type checker, and debugger inside itself. That had two downsides: the IDE only knew languages its vendor cared about, and your editor was locked in. If you liked Vim, you got mediocre language support; if you wanted JetBrains-level Java, you couldn't use Vim.

Microsoft cracked this in 2016 by extracting two open protocols:

- **LSP** — Language Server Protocol. Editors speak it. *Anyone* (Microsoft, Rust team, JetBrains, you) can ship a language server (like `clangd`, `rust-analyzer`, `pyright`) and every LSP-aware editor instantly supports that language.
- **DAP** — Debug Adapter Protocol. Same idea for debuggers. `lldb-vscode`, `debugpy`, `codelldb` each speak DAP; any DAP-aware editor gets step debugging for that language.

Now your editor can be Neovim, VS Code, Emacs, Helix, Zed, Sublime — they all get the same language intelligence by talking to the same servers. This lesson sets that up.

## The Concept

### LSP in one diagram

```
   ┌──────────────────┐    JSON-RPC over stdio    ┌─────────────────────┐
   │ Editor / IDE     │ ─────────────────────────▶│ Language server     │
   │  (Neovim, VS,    │ ◀──────────────────────── │  (clangd, rust-     │
   │   Helix, Emacs)  │                            │   analyzer, pyright,│
   │                  │                            │   tsserver, ...)   │
   └──────────────────┘                            └─────────────────────┘
       (LSP client)                                  (LSP server)
```

The editor knows nothing about Rust. It speaks LSP. The Rust server (`rust-analyzer`) does the heavy lifting: parsing, type checking, indexing, finding references, suggesting completions. The editor displays the responses.

**Common LSP requests** (you'll never see these directly; the editor does):

| Method | What it does |
|--------|--------------|
| `textDocument/didOpen` / `didChange` | "I opened/edited this file" |
| `textDocument/hover` | "What's at this position?" — type, docs |
| `textDocument/definition` | "Where is this symbol defined?" |
| `textDocument/references` | "Where is this symbol used?" |
| `textDocument/completion` | "Suggest completions at this position" |
| `textDocument/codeAction` | "Available refactors / fixes here" |
| `textDocument/publishDiagnostics` | Server → editor: "Here are the errors I just found" |

### DAP, the analog for debugging

DAP wraps gdb / lldb / debugpy / etc. behind a uniform JSON protocol. The editor sets breakpoints, steps, inspects locals — the adapter translates those into the debugger's own commands.

```
   ┌──────────────────┐    JSON-RPC        ┌──────────────────┐    native    ┌──────────┐
   │ Editor (DAP      │ ─────────────────▶ │ Debug adapter    │ ───────────▶ │ gdb/lldb │
   │ client)          │ ◀───────────────── │ (codelldb, etc.) │              └──────────┘
   └──────────────────┘                    └──────────────────┘
```

DAP is younger and less mature than LSP. Coverage is excellent for C/C++/Rust/Python/Node; spottier elsewhere.

### Servers you'll want for this course

| Language | LSP server | DAP adapter |
|----------|------------|-------------|
| C / C++  | `clangd`               | `codelldb` (LLDB) or `cpptools` (VS Code only) |
| Rust     | `rust-analyzer`        | `codelldb` |
| Python   | `pyright` or `ruff`    | `debugpy` |
| Go       | `gopls`                | `delve` |
| TypeScript | `typescript-language-server` | `js-debug` (Chrome devtools protocol-based) |
| Haskell  | `haskell-language-server` | `haskell-debug-adapter` |
| Markdown | `marksman` (optional)  | — |

All of these install via the editor's plugin manager (`mason.nvim` for Neovim; the VS Code extension store for VS Code).

### Why VS Code "just works" but Neovim asks you to configure things

VS Code ships an opinionated UI and an extension model where each language extension is a curated bundle (LSP server + DAP adapter + snippets + grammar). You install one extension; everything wires up.

Neovim (and Helix, Emacs) prefer composable configuration. `nvim-lspconfig` knows the *defaults* for ~150 servers; you say `lspconfig.clangd.setup{}` and it starts the server when you open a `.c`. DAP comes via `nvim-dap` + `nvim-dap-ui`.

The right framing: VS Code is the fast on-ramp, Neovim is the long-haul setup. Either works.

## Build It

### Step 1: Install the language servers (cross-editor)

`outputs/install-lsp-servers.sh` installs the servers you need. Pick a subset:

```sh
# C/C++
brew install llvm                  # macOS  — provides clangd
sudo apt install -y clangd         # Debian

# Rust
rustup component add rust-analyzer

# Python
pipx install pyright
pipx install ruff

# Go
go install golang.org/x/tools/gopls@latest

# TypeScript
npm install -g typescript typescript-language-server
```

After install, each server should answer to `--help` or just print "starting ..." on stdin.

### Step 2: VS Code path (faster)

Install extensions (Cmd+Shift+P → "Install Extensions"):

- `llvm-vs-code-extensions.vscode-clangd` (C/C++)
- `rust-lang.rust-analyzer` (Rust)
- `ms-python.python` + `charliermarsh.ruff` (Python)
- `vadimcn.vscode-lldb` (codelldb — provides DAP for C/C++/Rust)

Open the course repo: `code .`. The status bar should show "indexing" briefly per language. Open `phases/00-setup-and-tooling/04-c-c-toolchain.../code/main.c`, hover over `printf` — you should see its signature.

To debug:
1. Open the project; press F5; if no `launch.json` exists, VS Code offers to make one.
2. Pick the language ("LLDB" for C/Rust).
3. Set a breakpoint by clicking left of a line number.
4. F5 again to run; F10 to step over; F11 to step into; F5 to continue.

### Step 3: Neovim path (more setup, more control)

Use a starter config like `LazyVim` or `kickstart.nvim`. From scratch:

```lua
-- ~/.config/nvim/init.lua (abbreviated)

vim.cmd.colorscheme("default")

-- Bootstrap lazy.nvim (plugin manager)
local lazypath = vim.fn.stdpath("data") .. "/lazy/lazy.nvim"
if not vim.loop.fs_stat(lazypath) then
  vim.fn.system({ "git", "clone", "--filter=blob:none",
    "https://github.com/folke/lazy.nvim.git", "--branch=stable", lazypath })
end
vim.opt.rtp:prepend(lazypath)

require("lazy").setup({
  { "neovim/nvim-lspconfig" },
  { "williamboman/mason.nvim", build = ":MasonUpdate" },
  { "williamboman/mason-lspconfig.nvim" },
  { "hrsh7th/nvim-cmp", dependencies = { "hrsh7th/cmp-nvim-lsp" } },
  { "mfussenegger/nvim-dap" },
  { "rcarriga/nvim-dap-ui" },
})

require("mason").setup()
require("mason-lspconfig").setup({
  ensure_installed = { "clangd", "rust_analyzer", "pyright", "gopls" },
})

local lspconfig = require("lspconfig")
for _, server in ipairs({ "clangd", "rust_analyzer", "pyright", "gopls" }) do
  lspconfig[server].setup({})
end
```

Open a file; the language server starts automatically.

Useful Neovim LSP commands (default keymaps with `nvim-lspconfig`):

| Key | What it does |
|-----|--------------|
| `gd` | Go to definition |
| `gr` | Find references |
| `K`  | Hover (type / docs) |
| `<leader>rn` | Rename |
| `<leader>ca` | Code action (refactor / quick fix) |
| `[d` / `]d` | Previous / next diagnostic |

### Step 4: Verify the wiring

Open the lesson's `main.c`. You should see:

1. Inline red squiggles when you introduce a syntax error.
2. Hover showing `int printf(const char *format, ...)`.
3. `gd` jumping to `stdio.h`'s declaration of `printf`.
4. Auto-completion as you type `prin<TAB>`.

If any of those don't work, see Step 6.

### Step 5: Step debugging in DAP

In Neovim (with `nvim-dap`), in `lua/dap-config.lua`:

```lua
local dap = require("dap")

dap.adapters.lldb = {
  type = "executable",
  command = "/usr/bin/lldb-vscode",   -- or path to codelldb
  name = "lldb",
}

dap.configurations.c = {
  {
    name = "Launch",
    type = "lldb",
    request = "launch",
    program = function()
      return vim.fn.input("Path to executable: ", vim.fn.getcwd() .. "/", "file")
    end,
    cwd = "${workspaceFolder}",
    stopOnEntry = false,
    args = {},
  },
}
dap.configurations.rust = dap.configurations.c
dap.configurations.cpp = dap.configurations.c
```

In VS Code, the `vscode-lldb` extension supplies the same setup via a one-line `launch.json`.

Set a breakpoint, press F5 (or `:lua require'dap'.continue()`), step with `n`, inspect with the DAP UI.

### Step 6: Diagnosing LSP failures

When LSP "isn't working":

1. **Is the server installed?** `which clangd` / `which rust-analyzer`. If missing, install.
2. **Did the editor start it?** Neovim: `:LspInfo`. VS Code: status bar → "Output: clangd".
3. **Read the server log.** Neovim: `:LspLog`. VS Code: `Cmd-Shift-P` → "clangd: Show clangd output". Errors are usually obvious (missing `compile_commands.json`, wrong workspace root, etc.).
4. **Try the server directly.** Run `clangd --check=main.c` from the project root — does it produce the expected analysis?
5. **Check the LSP client.** Neovim: `:checkhealth lsp`. VS Code: "Developer: Open Logs Folder".

90% of "LSP broken" issues are a missing `compile_commands.json` (C/C++), a missing `rust-toolchain.toml` (Rust), or the editor opened a *subdirectory* instead of the project root.

## Use It

The same LSP setup powers:

- **Code review tooling.** GitHub's web editor (and Codespaces) speak LSP through a remote server.
- **AI coding agents.** Many AI coding agents use LSP behind the scenes to fetch context (definitions, references) before generating code.
- **Code-server / Coder / Gitpod.** Cloud IDEs run an LSP server on a remote VM and proxy it to your browser.

The protocol is the contract; the editor and the server can be swapped independently.

## Read the Source

- [Language Server Protocol Specification](https://microsoft.github.io/language-server-protocol/) — the full spec; readable in an afternoon.
- [Debug Adapter Protocol Specification](https://microsoft.github.io/debug-adapter-protocol/) — analog for debugging.
- [`rust-analyzer` architecture](https://github.com/rust-lang/rust-analyzer/blob/master/docs/dev/architecture.md) — a sterling example of a modern LSP server.
- [`nvim-lspconfig` server defaults](https://github.com/neovim/nvim-lspconfig/tree/master/lua/lspconfig/configs) — the wiring for every server.

## Ship It

This lesson ships **`outputs/install-lsp-servers.sh`** (cross-platform installer) and **`outputs/devcontainer.json.snippet`** (a snippet for `.devcontainer/devcontainer.json` that pre-installs the LSP servers when someone opens the repo in a dev container).

## Exercises

1. **Easy.** Install `clangd` and `rust-analyzer`. In either editor, open a course `.c` and `.rs` file and verify hover and go-to-definition work.
2. **Medium.** For the Phase 0, Lesson 04 `code/` folder, generate a `compile_commands.json` (via `bear -- make` or CMake's `CMAKE_EXPORT_COMPILE_COMMANDS`). Confirm clangd picks it up and stops showing "file not part of any project."
3. **Hard.** Configure DAP in your editor to debug both the C binary from Lesson 04 *and* a Rust binary from Lesson 05, sharing the same `codelldb` adapter. Document any platform-specific quirks (path to `codelldb`, etc.).

## Key Terms

| Term | What people say | What it actually means |
|------|----------------|------------------------|
| LSP | "Language Server Protocol" | A JSON-RPC spec that decouples editor from language tooling; one server can serve every editor |
| DAP | "Debug Adapter Protocol" | The same idea for debuggers |
| Language server | "The plugin" | A separate process the editor talks to over stdio; it does parsing, type checking, indexing |
| `compile_commands.json` | "C config" | A JSON list of every `(file, working dir, command)` your build runs; clangd uses it to know how to compile each file |

## Further Reading

- [Build Your Own Language Server (Microsoft tutorial)](https://code.visualstudio.com/api/language-extensions/language-server-extension-guide) — write a tiny LSP server in Node.
- [Helix Editor's LSP docs](https://docs.helix-editor.com/lang-support.html) — see another editor's take on the same protocol.
- [`bear`](https://github.com/rizsotto/Bear) — the tool that captures `compile_commands.json` from a Makefile build.
