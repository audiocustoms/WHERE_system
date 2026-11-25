# WHERE - does my stuff go?

Module for orders, package slips and the rental/sale prices on articles.

## Installation

```powershell
cargo run -p where-server --bin where-installer -- --module go
```

The installer checks the module registry (`where_modules`):

- If `WHERE - is my stuff?` (`stuff`) is installed, the module uses its
  database and tenants.
- If the core application is missing, the module creates the database itself and
  is registered as `database_owner`. When the core application is installed later,
  it takes over exactly this database.

## Contents

| Path | Contents |
| --- | --- |
| `module.toml` | Manifest (id, product name, capabilities) |
| `web/menu.ts` | Menus `Auftrag`, the package-slip entries in the `Lager` menu, admin entry, tiles |
| `web/OrdersAdminPage.svelte` | Order settings (schedule, headings, macros) |
| `web/ArticleRental.svelte`, `web/ArticleSale.svelte` | `Miete` and `Verkauf` tabs of the article form |
| `web/search/*` | Search forms for order and package slip |
| `crates/where-module-go` | Backend: `/api/v1/orders/*`, `/api/v1/master-data/scales*` |

Without the installed module the core application hides the `Miete`/`Verkauf`
tabs, the category checkboxes, the stock columns and the order menus. The backend
rejects the endpoints with `409` and leaves existing price/flag data unchanged
when an article is saved.
