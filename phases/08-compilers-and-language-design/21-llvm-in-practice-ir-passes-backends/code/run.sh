#!/usr/bin/env bash
set -euo pipefail

echo "LLVM workflow sketch:"
echo "- parse source into LLVM IR"
echo "- run optimization passes (instcombine, gvn, dce)"
echo "- emit target machine code via backend"
