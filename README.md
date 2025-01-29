ledger_ls is a language server for ledger file

## Getting Started

### Prerequisites

You need **rust** installed to be able to build

### Installation

1. Clone and build the repo

```sh
git clone https://github.com/DisezZ/ledger_ls
cd ledger_ls
cargo build
```

## Usage

### Neovim

To use ledger_ls with neovim, you should be able to setup via **lspconfig**

```lua
local lspconfig = require("lspconfig")
local configs = require("lspconfig.configs")
if not configs.ledger_ls then
    configs.ledger_ls = {
        default_config = {
            cmd = { "<path-to-your-executable>" },
            filetypes = { "ledger" },
            root_dir = lspconfig_util.root_pattern(".git") or vim.fn.getcwd(),
        },
    }
end
lspconfig.ledger_ls.setup({
    capabilities = capabilities,
    on_attach = on_attach,
})
```

## Features

- [x] Autocomplete
  - [x] Account
  - [x] Payee
  - [ ] Date
- [ ] Rename
- [ ] Formatting
- [ ] Diagnostics
- [ ] Definitions
- [ ] Hover
