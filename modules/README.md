# Modules of the WHERE family

Each subdirectory is a product of its own with a manifest (`module.toml`),
documentation and a Svelte front end. The backend lives in the crate of the same
name under `crates/`.

| Module | Product | Crate | Capabilities |
| --- | --- | --- | --- |
| `where-does-my-stuff-go` | WHERE - does my stuff go? | `where-module-go` | Orders, package slips, rental/sale prices |
| `where-are-my-people` | WHERE - are my people? | `where-module-people` | Personnel master |
| `where-is-my-money` | WHERE - is my money? | `where-module-money` | Accounting, banks, invoices |

The core application `WHERE - is my stuff?` stays under `apps/` and `crates/where-*`.
All products register in the same database (`where_modules`,
`where_installation`); the product installed first creates the database, and
every later one keeps using it together with its tenants.

Installation and status:

```powershell
cargo run -p where-server --bin where-installer -- --status
cargo run -p where-server --bin where-installer -- --module go
cargo run -p where-server --bin where-installer -- --remove money
```
