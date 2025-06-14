#!/usr/bin/env bash

set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
LSP_NAME="phppp"
LSP_BIN="$PROJECT_ROOT/target/release/$LSP_NAME"
EXT_DIR="$PROJECT_ROOT/.lsp-vscode-client"

# Detect editor (Cursor or VSCode)
if command -v cursor >/dev/null 2>&1; then
    EDITOR_CMD="cursor"
    echo "🎯 Detected Cursor editor"
elif command -v code >/dev/null 2>&1; then
    EDITOR_CMD="code"
    echo "🎯 Detected VSCode editor"
else
    echo "❌ Neither Cursor nor VSCode found in PATH"
    exit 1
fi

echo "🔨 Building Rust LSP..."
cargo build --release

echo "📦 Setting up VSCode extension in $EXT_DIR..."
mkdir -p "$EXT_DIR"

cat > "$EXT_DIR/package.json" <<EOF
{
  "name": "$LSP_NAME",
  "displayName": "PHP LSP (phppp)",
  "description": "Rust-based PHP Language Server Protocol implementation",
  "version": "0.0.1",
  "publisher": "local-dev",
  "main": "./extension.js",
  "engines": { "vscode": "^1.80.0" },
  "categories": ["Programming Languages", "Language Packs"],
  "activationEvents": ["onLanguage:php"],
  "repository": {
    "type": "git",
    "url": "https://github.com/thethongngu/phppp"
  },
  "license": "MIT",
  "keywords": ["php", "lsp", "language server", "phppp"],
  "contributes": {
    "languages": [{ 
      "id": "php", 
      "extensions": [".php"],
      "aliases": ["PHP", "php"]
    }]
  },
  "devDependencies": {
    "@vscode/vsce": "^2.22.0"
  }
}
EOF

cat > "$EXT_DIR/extension.js" <<EOF
const path = require("path");
const fs = require("fs");
const vscode = require("vscode");
const { LanguageClient } = require("vscode-languageclient/node");

function activate(context) {
  // Try multiple possible locations for the LSP binary
  const possiblePaths = [
    "$PROJECT_ROOT/target/release/$LSP_NAME",
    path.resolve(__dirname, "../target/release/$LSP_NAME"),
    path.resolve(__dirname, "../../target/release/$LSP_NAME")
  ];
  
  let serverExe = null;
  for (const p of possiblePaths) {
    if (fs.existsSync(p)) {
      serverExe = p;
      break;
    }
  }
  
  if (!serverExe) {
    vscode.window.showErrorMessage("$LSP_NAME: LSP server binary not found. Please rebuild with: cargo build --release");
    return;
  }

  const client = new LanguageClient(
    "$LSP_NAME",
    "$LSP_NAME",
    { command: serverExe },
    {
      documentSelector: [{ scheme: "file", language: "php" }],
      synchronize: {
        fileEvents: vscode.workspace.createFileSystemWatcher("**/*.php")
      }
    }
  );

  context.subscriptions.push(client.start());
}
exports.activate = activate;
EOF

echo "📦 Installing npm dependencies..."
cd "$EXT_DIR"

# Always install/update dependencies
npm install vscode-languageclient @vscode/vsce

echo "🧪 Packaging extension..."
npx vsce package --no-yarn --pre-release -o "$PROJECT_ROOT/$LSP_NAME.vsix"

echo "📥 Installing into $EDITOR_CMD..."
# Uninstall existing version first (if any)
$EDITOR_CMD --uninstall-extension "$LSP_NAME" 2>/dev/null || true

# Install new version
$EDITOR_CMD --install-extension "$PROJECT_ROOT/$LSP_NAME.vsix" --force

echo "🔍 Verifying extension installation..."
if $EDITOR_CMD --list-extensions | grep -q "^$LSP_NAME$"; then
    echo "✅ Extension '$LSP_NAME' is installed successfully."
    echo "💡 If the extension doesn't activate automatically, restart $EDITOR_CMD."
    echo "💡 Then open a .php file to trigger the LSP server."
    echo "💡 Check $EDITOR_CMD Output panel → '$LSP_NAME' for server logs."
else
    echo "❌ Extension installation may have failed."
    echo "💡 Try restarting $EDITOR_CMD and check the Extensions view."
fi