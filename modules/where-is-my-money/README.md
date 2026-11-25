# WHERE - is my money?

Module for accounting and financial bookkeeping: bank list (SWIFT/BIC), the
company's bank accounts and the search forms of the accounting department.

## Installation

```powershell
cargo run -p where-server --bin where-installer -- --module money
```

The module uses the database and the tenants of `WHERE - is my stuff?` when the
core application is installed. Otherwise it creates the database itself; the core
application takes it over on a later installation.

## Contents

| Path | Contents |
| --- | --- |
| `module.toml` | Manifest (id, product name, capabilities) |
| `web/menu.ts` | Menu `Faktura`, admin entries, tiles |
| `web/BankListPage.svelte` | Bank list (`/admin/banks`) |
| `web/BankAccountsPage.svelte` | Bank accounts (`/admin/bank-accounts`) |
| `web/search/*` | Invoice and incoming-invoice search |
| `crates/where-module-money` | Backend: `/api/v1/banks*`, `/api/v1/bank-accounts*` |
