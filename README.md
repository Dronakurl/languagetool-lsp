# languagetool-lsp

A Language Server Protocol (LSP) implementation that provides grammar and spell checking using [LanguageTool](https://www.languagetool.org/).

## Features

- **Real-time grammar and spell checking** for text documents via LSP
- **Auto-detection** of 30+ languages (no manual language specification needed)
- **Smart code actions** for corrections with cursor-position-aware suggestions
- **Optional language override** via comments when auto-detection needs help
- **Pure Rust implementation** with async communication to LanguageTool server
- **UTF-8 support** with proper character offset handling

## Prerequisites

- Rust toolchain (2021 edition or later)
- LanguageTool server (can be run via Docker)

### Running LanguageTool

**Using Docker:**
```bash
docker run -p 8010:8010 erikvl87/languagetool
```

The server will be available at `http://localhost:8010`.

## Installation

```bash
cargo install --path .
```

This installs the binary to `~/.cargo/bin/languagetool-lsp`.

## Usage

### As a Language Server

The LSP server communicates via stdin/stdout. Configure your editor to launch the binary:

```bash
~/.cargo/bin/languagetool-lsp
```

### Language Detection

**Automatic (Default):**
LanguageTool automatically detects the language from the text content with 99% accuracy.

**Manual Override:**
Specify the language by adding a `lang:` pattern anywhere in your document:

**Format:** `lang: xx-YY` where xx is language code and YY is country code

**Examples:**
- `<!-- lang: en-US -->` (American English)
- `# lang: de-DE` (German)
- `// lang: fr-FR` (French)
- `lang: es-ES` (Spanish)

**Comment styles supported:**
- HTML comments: `<!-- lang: xx-YY -->`
- Shell-style: `# lang: xx-YY`
- C-style: `// lang: xx-YY`
- INI-style: `; lang: xx-YY`
- LaTeX/TeX-style: `% lang: xx-YY`
- Plain text: `lang: xx-YY`

### Code Actions

The LSP server provides intelligent correction suggestions:

- **Cursor on misspelled word** → Shows corrections for that word only
- **Cursor elsewhere on line** → Shows corrections for all issues in the line
- **Severity** → Hints (not warnings) for less intrusive editing

## Editor Configuration

### Helix

Add to your `~/.config/helix/languages.toml`:

```toml
[language-server.languagetool-lsp]
command = "/home/user/.cargo/bin/languagetool-lsp"

[[language]]
name = "markdown"
language-servers = ["languagetool-lsp"]

[[language]]
name = "text"
language-servers = ["languagetool-lsp"]
```

### VS Code

Install the [vscode-lsp](https://marketplace.visualstudio.com/itemName= vadimcn.vscode-lsp) extension and add to your settings:

```json
{
  "lsp.languagetool-lsp.command": "/home/user/.cargo/bin/languagetool-lsp",
  "lsp.languagetool-lsp.args": [],
  "languages": {
    "markdown": {
      "languageServers": ["languagetool-lsp"]
    }
  }
}
```

## How It Works

1. The LSP server receives document changes from the editor
2. Extracts optional language specification from comments (or uses auto-detection)
3. Sends text to LanguageTool server for analysis
4. Processes matches and calculates proper character positions
5. Publishes diagnostics (hints) for grammar/spelling issues back to the editor
6. Provides code actions for applying corrections

## Architecture

- **LSP Protocol**: Standard Language Server Protocol implementation
- **Async Communication**: Uses tokio for async LanguageTool API calls
- **UTF-8 Handling**: Proper character offset calculation for international text
- **State Management**: Maintains per-document diagnostic state
- **Smart Code Actions**: Context-aware suggestions based on cursor position

## LanguageTool Features

LanguageTool provides much more than just spell checking:

- **Grammar checking**: Subject-verb agreement, proper verb forms
- **Style suggestions**: Passive voice, wordiness, clarity
- **Spelling corrections**: Context-aware spelling improvements
- **Punctuation**: Comma placement, quotation marks
- **30+ languages**: Comprehensive language support

## Development

### Building

```bash
cargo build --release
```

### Running Tests

```bash
cargo test
```

### Manual Testing

Test files are provided in the `tests/` directory:
- `debug_sentence.md` - Simple German sentence with errors
- `test_deutsche_texte.md` - German text without language specification
- `test_auto_detection.md` - Mixed language text for auto-detection
- `test_final.md` - Comprehensive test with multiple error types

Open any test file in your editor with the LSP configured to see the diagnostics.

## Contributing

Contributions are welcome! Please feel free to submit issues or pull requests.

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## Acknowledgments

- [LanguageTool](https://www.languagetool.org/) - The underlying grammar and spell checking engine
- [languagetool-rust](https://github.com/jeertmans/languagetool-rust) - Rust bindings for LanguageTool API
- [hunspell-lsp](https://github.com/Dronakurl/hunspell-lsp) - Inspiration for this project
