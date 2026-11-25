use chrono::Datelike;
use sqlx::{AssertSqlSafe, PgPool};
use where_domain::{Address, AddressInput, DomainError, Page};

use crate::article_search_repo::SetOperator;
use crate::error::db_error;
use crate::rows::AddressRow;

/// `WHERE` block of the address list, shared by the count and the page query.
///
/// `$3` quick search, `$4` state (`all`, `active`, `inactive`). An imported row
/// without a value in `"active"` counts as active.
const LIST_FILTER: &str = r#"
 WHERE ($3::text IS NULL
        OR "Name1" ILIKE '%' || $3 || '%'
        OR "Name2" ILIKE '%' || $3 || '%'
        OR "Name3" ILIKE '%' || $3 || '%'
        OR "KundenNr"::text ILIKE '%' || $3 || '%')
   AND ($4::text = 'all'
        OR ($4 = 'active' AND COALESCE("active", true))
        OR ($4 = 'inactive' AND NOT COALESCE("active", true)))"#;

// Sort expressions of the address list. Only one of these fixed literals is
// interpolated into the page query, never user input.
const SORT_EXPR_TYPE: &str = r#"array_to_string(ARRAY_REMOVE(ARRAY[
        CASE WHEN COALESCE("Type_Kunde", false) THEN 'K' END,
        CASE WHEN COALESCE("Type_Interessent", false) THEN 'I' END,
        CASE WHEN COALESCE("Type_Lieferant", false) THEN 'L' END,
        CASE WHEN COALESCE("Type_Location", false) THEN 'Loc' END,
        CASE WHEN COALESCE("Type_VIP", false) THEN 'VIP' END], NULL), ' ')"#;
const SORT_EXPR_NUMBER: &str = r#"COALESCE("KundenNr", "lief_LieferantenNr")"#;
const SORT_EXPR_NAME: &str = r#""Name1""#;
const SORT_EXPR_COUNTRY: &str = r#""import_Land""#;
const SORT_EXPR_POSTAL_CODE: &str = r#""import_PLZ""#;
const SORT_EXPR_CITY: &str = r#""import_Ort""#;

const GET_SQL: &str = r#"SELECT * FROM "Adresse" WHERE "Primary_key" = $1"#;

const INSERT_SQL: &str = r#"
INSERT INTO "Adresse"
    ("KundenNr", "Titel", "Name1", "Name2", "Name3", "Anrede",
     "import_Strasse", "import_Land", "import_Ort", "USTID", "knd_Debitor",
     "import_Telefon", "import_Email", "GruppenNameAdresse",
     "Type_Kunde", "Type_Lieferant", "Auftragssperre", "Mahnsperre",
     "active", "adr_Einzelperson", "Person_Geburtstag", "import_Adresszusatz",
     "import_PLZ", "import_Bundesland", "import_Fax", "import_MobilTel",
     "import_URL", "Type_Interessent", "Type_Location", "Type_Personal",
     "Type_VIP", "Adr_EORINr", "Knd_Steuernummer", "HRB", "HRB_Sitz",
     "KreditLimit", "import_knd_RabattMiete", "import_knd_RabattVerkauf",
     "knd_Bank", "knd_Konto", "knd_BLZ", "knd_IBAN", "knd_BIC",
     "lief_LieferantenNr", "lief_Kreditor", "lief_Umsatz",
     "import_lief_RabattMiete", "import_lief_RabattVerkauf",
     "lief_Bank", "lief_Konto", "lief_BLZ", "lief_IBAN", "lief_BIC")
VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14,
        $15, $16, $17, $18, $19, $20, $21, $22, $23, $24, $25, $26,
        $27, $28, $29, $30, $31, $32, $33, $34, $35, $36, $37, $38,
        $39, $40, $41, $42, $43, $44, $45, $46, $47, $48, $49, $50,
        $51, $52, $53)
RETURNING *"#;

/// Columns of the update; the numbers continue after the `$1` of the key.
const UPDATE_SQL: &str = r#"
UPDATE "Adresse" SET
    "KundenNr" = $2, "Titel" = $3, "Name1" = $4, "Name2" = $5, "Name3" = $6,
    "Anrede" = $7, "import_Strasse" = $8, "import_Land" = $9, "import_Ort" = $10,
    "USTID" = $11, "knd_Debitor" = $12, "import_Telefon" = $13,
    "import_Email" = $14, "GruppenNameAdresse" = $15, "Type_Kunde" = $16,
    "Type_Lieferant" = $17, "Auftragssperre" = $18, "Mahnsperre" = $19,
    "active" = $20, "adr_Einzelperson" = $21, "Person_Geburtstag" = $22,
    "import_Adresszusatz" = $23, "import_PLZ" = $24, "import_Bundesland" = $25,
    "import_Fax" = $26, "import_MobilTel" = $27, "import_URL" = $28,
    "Type_Interessent" = $29, "Type_Location" = $30, "Type_Personal" = $31,
    "Type_VIP" = $32, "Adr_EORINr" = $33, "Knd_Steuernummer" = $34,
    "HRB" = $35, "HRB_Sitz" = $36, "KreditLimit" = $37,
    "import_knd_RabattMiete" = $38, "import_knd_RabattVerkauf" = $39,
    "knd_Bank" = $40, "knd_Konto" = $41, "knd_BLZ" = $42, "knd_IBAN" = $43,
    "knd_BIC" = $44, "lief_LieferantenNr" = $45, "lief_Kreditor" = $46,
    "lief_Umsatz" = $47, "import_lief_RabattMiete" = $48,
    "import_lief_RabattVerkauf" = $49, "lief_Bank" = $50, "lief_Konto" = $51,
    "lief_BLZ" = $52, "lief_IBAN" = $53, "lief_BIC" = $54
WHERE "Primary_key" = $1
RETURNING *"#;

const DELETE_SQL: &str = r#"DELETE FROM "Adresse" WHERE "Primary_key" = $1"#;

/// A customer number must not exist twice (`$2` = the row being replaced, 0 = new).
const CUSTOMER_NO_TAKEN_SQL: &str =
    r#"SELECT 1 FROM "Adresse" WHERE "KundenNr" = $1 AND "Primary_key" <> $2 LIMIT 1"#;

/// Same for the supplier number of the supplier tab.
const SUPPLIER_NO_TAKEN_SQL: &str =
    r#"SELECT 1 FROM "Adresse" WHERE "lief_LieferantenNr" = $1 AND "Primary_key" <> $2 LIMIT 1"#;

/// Duplicate check of the "Dublettenverhinderer" window: one term, broad match.
const DUPLICATES_SQL: &str = r#"
SELECT * FROM "Adresse"
 WHERE ($2::text IS NULL
        OR "Name1" ILIKE '%' || $2 || '%'
        OR "Name2" ILIKE '%' || $2 || '%'
        OR "Name3" ILIKE '%' || $2 || '%'
        OR "Name_komplett" ILIKE '%' || $2 || '%'
        OR "import_Strasse" ILIKE '%' || $2 || '%'
        OR "import_Ort" ILIKE '%' || $2 || '%'
        OR "import_Email" ILIKE '%' || $2 || '%'
        OR "import_Telefon" ILIKE '%' || $2 || '%'
        OR "KundenNr"::text = $2
        OR "knd_Debitor" = $2)
 ORDER BY "Name1", "Primary_key"
 LIMIT $1"#;

const DUPLICATES_COUNT_SQL: &str = r#"
SELECT count(*) FROM "Adresse"
 WHERE ($1::text IS NULL
        OR "Name1" ILIKE '%' || $1 || '%'
        OR "Name2" ILIKE '%' || $1 || '%'
        OR "Name3" ILIKE '%' || $1 || '%'
        OR "Name_komplett" ILIKE '%' || $1 || '%'
        OR "import_Strasse" ILIKE '%' || $1 || '%'
        OR "import_Ort" ILIKE '%' || $1 || '%'
        OR "import_Email" ILIKE '%' || $1 || '%'
        OR "import_Telefon" ILIKE '%' || $1 || '%'
        OR "KundenNr"::text = $1
        OR "knd_Debitor" = $1)"#;

/// Distinct values of the address search index ("Index wählen" dialog).
const INDEX_VALUES_SQL: &str = r#"
SELECT DISTINCT "Adressindex" AS name
  FROM "Adressindex"
 WHERE "Adressindex" IS NOT NULL AND btrim("Adressindex") <> ''
 ORDER BY "Adressindex""#;

/// Which addresses the search returns ("Typ" of the search form).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AddressType {
    #[default]
    All,
    Customers,
    Suppliers,
}

impl AddressType {
    pub fn key(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Customers => "customers",
            Self::Suppliers => "suppliers",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "all" => Some(Self::All),
            "customers" => Some(Self::Customers),
            "suppliers" => Some(Self::Suppliers),
            _ => None,
        }
    }
}

/// "Bestand" switch above the address list.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AddressState {
    All,
    #[default]
    Active,
    Inactive,
}

impl AddressState {
    pub fn key(self) -> &'static str {
        match self {
            Self::All => "all",
            Self::Active => "active",
            Self::Inactive => "inactive",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "all" => Some(Self::All),
            "active" => Some(Self::Active),
            "inactive" => Some(Self::Inactive),
            _ => None,
        }
    }
}

/// Sortable columns of the address list.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum AddressSort {
    /// Role codes (`K`, `I`, `L`, `Loc`, `VIP`) as shown in the "Typ" column.
    Type,
    /// Customer number, for suppliers without one their supplier number.
    Number,
    #[default]
    Name,
    Country,
    PostalCode,
    City,
}

impl AddressSort {
    pub fn key(self) -> &'static str {
        match self {
            Self::Type => "type",
            Self::Number => "number",
            Self::Name => "name",
            Self::Country => "country",
            Self::PostalCode => "postal_code",
            Self::City => "city",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "type" => Some(Self::Type),
            "number" => Some(Self::Number),
            "name" => Some(Self::Name),
            "country" => Some(Self::Country),
            "postal_code" => Some(Self::PostalCode),
            "city" => Some(Self::City),
            _ => None,
        }
    }

    /// Fixed `ORDER BY` expression of the column; never user input.
    pub fn order_expr(self) -> &'static str {
        match self {
            Self::Type => SORT_EXPR_TYPE,
            Self::Number => SORT_EXPR_NUMBER,
            Self::Name => SORT_EXPR_NAME,
            Self::Country => SORT_EXPR_COUNTRY,
            Self::PostalCode => SORT_EXPR_POSTAL_CODE,
            Self::City => SORT_EXPR_CITY,
        }
    }
}

/// Query of the address list behind the "Adressen" page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AddressList {
    pub limit: i64,
    pub offset: i64,
    /// Quick search over the three name lines and the customer number.
    pub search: Option<String>,
    pub state: AddressState,
    pub sort: AddressSort,
    pub descending: bool,
}

impl Default for AddressList {
    fn default() -> Self {
        Self {
            limit: 50,
            offset: 0,
            search: None,
            state: AddressState::default(),
            sort: AddressSort::default(),
            descending: false,
        }
    }
}

/// Matching mode of the "Suchtext enthält…" control in the window bar.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TextMatch {
    #[default]
    Contains,
    StartsWith,
    Equals,
}

impl TextMatch {
    pub fn key(self) -> &'static str {
        match self {
            Self::Contains => "contains",
            Self::StartsWith => "starts_with",
            Self::Equals => "equals",
        }
    }

    pub fn from_key(key: &str) -> Option<Self> {
        match key {
            "contains" => Some(Self::Contains),
            "starts_with" => Some(Self::StartsWith),
            "equals" => Some(Self::Equals),
            _ => None,
        }
    }

    /// `ILIKE` pattern for a term; `equals` keeps the term without wildcards.
    fn pattern(self, term: &str) -> String {
        match self {
            Self::Contains => format!("%{term}%"),
            Self::StartsWith => format!("{term}%"),
            Self::Equals => term.to_string(),
        }
    }
}

/// Criteria of the address search form (`img/suche/Adresssuche.PNG`).
///
/// All fields are optional; the criteria that are set are combined with AND.
#[derive(Debug, Clone, Default)]
pub struct AddressSearch {
    pub address_type: AddressType,
    /// "Name" — matches the three name lines, the full name and contact persons.
    pub name: Option<String>,
    /// Matching mode of the name and address text fields.
    pub name_match: TextMatch,
    /// "inklusive Ansprechpartner" — also search the contact persons of an address.
    pub include_contacts: bool,
    /// "Adressgruppe"
    pub group: Option<String>,
    /// "Knd. / Debitor-Nr." — `0` and empty mean "any".
    pub customer_no: Option<i32>,
    /// "Knd. / Debitor-Nr." — the debtor number as text.
    pub debtor_no: Option<String>,
    /// "Anschrift" — street, postal code, city and country.
    pub address_text: Option<String>,
    /// "Telefon"
    pub phone: Option<String>,
    /// "eMail"
    pub email: Option<String>,
    /// "Suchindex" — values of `"Adressindex"."Adressindex"`.
    pub search_index: Vec<String>,
    pub search_index_op: SetOperator,
    /// Second "inklusive Ansprechpartner": index entries of a contact person
    /// (`"Adressindex"."AdressAP_Key"`) also count.
    pub include_contact_index: bool,
}

/// Empty text fields count as "not set", the number 0 as "any".
fn text(value: Option<String>) -> Option<String> {
    value
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
}

impl AddressSearch {
    /// Cleans the input: drop empty texts, drop the number 0, remove duplicates.
    pub fn normalized(mut self) -> Self {
        self.name = text(self.name);
        self.group = text(self.group);
        self.debtor_no = text(self.debtor_no);
        self.address_text = text(self.address_text);
        self.phone = text(self.phone);
        self.email = text(self.email);
        self.customer_no = self.customer_no.filter(|n| *n > 0);
        self.search_index = std::mem::take(&mut self.search_index)
            .into_iter()
            .filter_map(|s| text(Some(s)))
            .collect();
        self.search_index.sort();
        self.search_index.dedup();
        self
    }
}

/// `WHERE` block of the address search; used by the count and the page query.///
/// `$3` type, `$4` name pattern, `$5` include contacts, `$6` group pattern,
/// `$7` customer no., `$8` debtor pattern, `$9` address pattern, `$10` phone
/// pattern, `$11` e-mail pattern, `$12` search index, `$13` index operator,
/// `$14` include contact index entries.
const SEARCH_FILTER: &str = r#"
 WHERE ($3::text = 'all'
        OR ($3 = 'customers' AND COALESCE(a."Type_Kunde", false))
        OR ($3 = 'suppliers' AND COALESCE(a."Type_Lieferant", false)))
   AND ($4::text IS NULL
        OR a."Name1" ILIKE $4
        OR a."Name2" ILIKE $4
        OR a."Name3" ILIKE $4
        OR a."Name_komplett" ILIKE $4
        OR ($5::bool AND EXISTS (
              SELECT 1 FROM "AdressAP" ap
               WHERE ap."Adress_Key" = a."Primary_key"
                 AND (ap."Nachname" ILIKE $4
                   OR ap."Vorname" ILIKE $4
                   OR ap."Name_komplett" ILIKE $4))))
   AND ($6::text IS NULL OR a."GruppenNameAdresse" ILIKE $6)
   AND ($7::int IS NULL OR a."KundenNr" = $7)
   AND ($8::text IS NULL OR a."knd_Debitor" ILIKE $8)
   AND ($9::text IS NULL
        OR a."import_Strasse" ILIKE $9
        OR a."import_PLZ" ILIKE $9
        OR a."import_Ort" ILIKE $9
        OR a."import_Land" ILIKE $9)
   AND ($10::text IS NULL
        OR a."import_Telefon" ILIKE $10
        OR a."import_MobilTel" ILIKE $10)
   AND ($11::text IS NULL
        OR a."import_Email" ILIKE $11
        OR a."import_EmailInvoice" ILIKE $11
        OR ($5::bool AND EXISTS (
              SELECT 1 FROM "AdressAP" ap
               WHERE ap."Adress_Key" = a."Primary_key"
                 AND (ap."import_Email" ILIKE $11 OR ap."Email2" ILIKE $11))))
   AND ($12::text[] IS NULL OR CASE $13::text
          WHEN 'contains_all' THEN
               (SELECT count(DISTINCT i."Adressindex")
                  FROM "Adressindex" i
                 WHERE i."Adress_Key" = a."Primary_key"
                   AND i."Adressindex" = ANY($12)
                   AND ($14::bool OR i."AdressAP_Key" IS NULL)) = cardinality($12)
          WHEN 'contains_none' THEN NOT EXISTS (
               SELECT 1 FROM "Adressindex" i
                WHERE i."Adress_Key" = a."Primary_key"
                  AND i."Adressindex" = ANY($12)
                  AND ($14::bool OR i."AdressAP_Key" IS NULL))
          ELSE EXISTS (
               SELECT 1 FROM "Adressindex" i
                WHERE i."Adress_Key" = a."Primary_key"
                  AND i."Adressindex" = ANY($12)
                  AND ($14::bool OR i."AdressAP_Key" IS NULL))
        END)"#;

/// Repository for the `"Adresse"` master data table.
#[derive(Clone)]
pub struct AddressRepository {
    pool: PgPool,
}

/// Binds the columns of `INSERT_SQL` / `UPDATE_SQL` in their declared order.
///
/// Both statements share the list, so a single place keeps the 53 parameters in
/// sync with the SQL; a shift here is what `verify-schema.mjs` catches.
macro_rules! bind_columns {
    ($query:expr, $a:expr) => {
        $query
            .bind($a.customer_no)
            .bind($a.title.as_deref())
            .bind($a.name1.as_deref())
            .bind($a.name2.as_deref())
            .bind($a.name3.as_deref())
            .bind($a.salutation.as_deref())
            .bind($a.street.as_deref())
            .bind($a.country.as_deref())
            .bind($a.city.as_deref())
            .bind($a.vat_id.as_deref())
            .bind($a.debtor.as_deref())
            .bind($a.phone.as_deref())
            .bind($a.email.as_deref())
            .bind($a.group.as_deref())
            .bind($a.is_customer)
            .bind($a.is_supplier)
            .bind($a.auftragssperre)
            .bind($a.mahnsperre)
            .bind($a.active)
            .bind($a.is_person)
            .bind($a.birthday)
            .bind($a.address_addition.as_deref())
            .bind($a.postal_code.as_deref())
            .bind($a.state.as_deref())
            .bind($a.fax.as_deref())
            .bind($a.mobile.as_deref())
            .bind($a.website.as_deref())
            .bind($a.is_interested)
            .bind($a.is_location)
            .bind($a.is_employee)
            .bind($a.is_vip)
            .bind($a.eori_no.as_deref())
            .bind($a.tax_number.as_deref())
            .bind($a.register_entry.as_deref())
            .bind($a.register_seat.as_deref())
            .bind($a.credit_limit)
            .bind($a.discount_rental)
            .bind($a.discount_sale)
            .bind($a.bank.as_deref())
            .bind($a.bank_account.as_deref())
            .bind($a.bank_code.as_deref())
            .bind($a.iban.as_deref())
            .bind($a.bic.as_deref())
            .bind($a.supplier_no)
            .bind($a.creditor.as_deref())
            .bind($a.supplier_revenue)
            .bind($a.supplier_discount_rental)
            .bind($a.supplier_discount_purchase)
            .bind($a.supplier_bank.as_deref())
            .bind($a.supplier_bank_account.as_deref())
            .bind($a.supplier_bank_code.as_deref())
            .bind($a.supplier_iban.as_deref())
            .bind($a.supplier_bic.as_deref())
    };
}

impl AddressRepository {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// One page of the "Adressen" list: quick search, active filter and the
    /// sortable columns. Only fixed `ORDER BY` expressions are interpolated.
    pub async fn list(&self, query: &AddressList) -> Result<Page<Address>, DomainError> {
        let search = query
            .search
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty());
        let state = query.state.key();
        let direction = if query.descending { "DESC" } else { "ASC" };

        let count_sql = AssertSqlSafe(format!(
            "SELECT count(*) FROM \"Adresse\"{LIST_FILTER} LIMIT $1 OFFSET $2"
        ));
        let page_sql = AssertSqlSafe(format!(
            "SELECT * FROM \"Adresse\"{LIST_FILTER} \
             ORDER BY {} {direction} NULLS LAST, \"Primary_key\" LIMIT $1 OFFSET $2",
            query.sort.order_expr()
        ));

        let total: i64 = sqlx::query_scalar(count_sql)
            .bind(query.limit)
            .bind(query.offset)
            .bind(search)
            .bind(state)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;

        let rows = sqlx::query_as::<_, AddressRow>(page_sql)
            .bind(query.limit)
            .bind(query.offset)
            .bind(search)
            .bind(state)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;

        Ok(Page {
            items: rows.into_iter().map(Into::into).collect(),
            total,
            limit: query.limit,
            offset: query.offset,
        })
    }

    /// Runs the address search form (`img/suche/Adresssuche.PNG`).
    pub async fn search(
        &self,
        search: &AddressSearch,
        limit: i64,
        offset: i64,
    ) -> Result<Page<Address>, DomainError> {
        let name = search.name.as_deref().map(|n| search.name_match.pattern(n));
        let group = search
            .group
            .as_deref()
            .map(|g| TextMatch::Contains.pattern(g));
        let debtor = search
            .debtor_no
            .as_deref()
            .map(|d| TextMatch::Contains.pattern(d));
        let address_text = search
            .address_text
            .as_deref()
            .map(|t| search.name_match.pattern(t));
        let phone = search
            .phone
            .as_deref()
            .map(|p| TextMatch::Contains.pattern(p));
        let email = search
            .email
            .as_deref()
            .map(|e| TextMatch::Contains.pattern(e));
        let index = if search.search_index.is_empty() {
            None
        } else {
            Some(search.search_index.clone())
        };

        // `LIMIT`/`OFFSET` in the count statement only exist so both statements
        // bind the same parameter list. Only the fixed `SEARCH_FILTER` block is
        // interpolated, so no user input reaches the SQL text.
        let count_sql = AssertSqlSafe(format!(
            "SELECT count(*) FROM \"Adresse\" a{SEARCH_FILTER} LIMIT $1 OFFSET $2"
        ));
        let page_sql = AssertSqlSafe(format!(
            "SELECT * FROM \"Adresse\" a{SEARCH_FILTER} \
             ORDER BY a.\"Name1\", a.\"Primary_key\" LIMIT $1 OFFSET $2"
        ));

        let count = sqlx::query_scalar::<_, i64>(count_sql)
            .bind(limit)
            .bind(offset)
            .bind(search.address_type.key())
            .bind(&name)
            .bind(search.include_contacts)
            .bind(&group)
            .bind(search.customer_no)
            .bind(&debtor)
            .bind(&address_text)
            .bind(&phone)
            .bind(&email)
            .bind(&index)
            .bind(search.search_index_op.key())
            .bind(search.include_contact_index);
        let total: i64 = count.fetch_one(&self.pool).await.map_err(db_error)?;

        let rows = sqlx::query_as::<_, AddressRow>(page_sql)
            .bind(limit)
            .bind(offset)
            .bind(search.address_type.key())
            .bind(&name)
            .bind(search.include_contacts)
            .bind(&group)
            .bind(search.customer_no)
            .bind(&debtor)
            .bind(&address_text)
            .bind(&phone)
            .bind(&email)
            .bind(&index)
            .bind(search.search_index_op.key())
            .bind(search.include_contact_index)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;

        Ok(Page {
            items: rows.into_iter().map(Into::into).collect(),
            total,
            limit,
            offset,
        })
    }

    pub async fn get(&self, id: i32) -> Result<Address, DomainError> {
        let row = sqlx::query_as::<_, AddressRow>(GET_SQL)
            .bind(id)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?
            .ok_or(DomainError::NotFound {
                entity: "Address",
                id,
            })?;
        Ok(row.into())
    }

    pub async fn create(&self, input: &AddressInput) -> Result<Address, DomainError> {
        input.validate()?;
        self.ensure_numbers_free(input, None).await?;
        let row = bind_columns!(sqlx::query_as::<_, AddressRow>(INSERT_SQL), input)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;
        self.align_series(input).await?;
        Ok(row.into())
    }

    pub async fn update(&self, id: i32, input: &AddressInput) -> Result<Address, DomainError> {
        input.validate()?;
        self.ensure_numbers_free(input, Some(id)).await?;
        let row = bind_columns!(sqlx::query_as::<_, AddressRow>(UPDATE_SQL).bind(id), input)
            .fetch_optional(&self.pool)
            .await
            .map_err(db_error)?
            .ok_or(DomainError::NotFound {
                entity: "Address",
                id,
            })?;
        self.align_series(input).await?;
        Ok(row.into())
    }

    /// Keeps the `customer`/`supplier` series in step with numbers the mask sent by
    /// hand — otherwise the next form would offer the same, now taken, number again.
    async fn align_series(&self, input: &AddressInput) -> Result<(), DomainError> {
        let year = chrono::Utc::now().year();
        let mut conn = self.pool.acquire().await.map_err(db_error)?;
        if let Some(customer_no) = input.customer_no {
            crate::number_series_repo::align_on(&mut conn, "customer", customer_no, year)
                .await
                .map_err(db_error)?;
        }
        if let Some(supplier_no) = input.supplier_no {
            crate::number_series_repo::align_on(&mut conn, "supplier", supplier_no, year)
                .await
                .map_err(db_error)?;
        }
        Ok(())
    }

    /// The mask proposes the next number of the series; a number typed by hand
    /// must not collide with an existing address.
    async fn ensure_numbers_free(
        &self,
        input: &AddressInput,
        id: Option<i32>,
    ) -> Result<(), DomainError> {
        if let Some(customer_no) = input.customer_no {
            let taken = sqlx::query_scalar::<_, i32>(CUSTOMER_NO_TAKEN_SQL)
                .bind(customer_no)
                .bind(id.unwrap_or(0))
                .fetch_optional(&self.pool)
                .await
                .map_err(db_error)?;
            if taken.is_some() {
                return Err(DomainError::Conflict(format!(
                    "the customer number {customer_no} is already assigned"
                )));
            }
        }
        if let Some(supplier_no) = input.supplier_no {
            let taken = sqlx::query_scalar::<_, i32>(SUPPLIER_NO_TAKEN_SQL)
                .bind(supplier_no)
                .bind(id.unwrap_or(0))
                .fetch_optional(&self.pool)
                .await
                .map_err(db_error)?;
            if taken.is_some() {
                return Err(DomainError::Conflict(format!(
                    "the supplier number {supplier_no} is already assigned"
                )));
            }
        }
        Ok(())
    }

    /// Duplicate check of the "Dublettenverhinderer" window.
    ///
    /// One term, matched broadly (`Name1..3`, `Name_komplett`, street, city, e-mail,
    /// phone, customer and debtor number) so a misspelling of the same
    /// address still shows up.
    pub async fn duplicates(
        &self,
        term: Option<&str>,
        limit: i64,
    ) -> Result<Page<Address>, DomainError> {
        let total: i64 = sqlx::query_scalar(DUPLICATES_COUNT_SQL)
            .bind(term)
            .fetch_one(&self.pool)
            .await
            .map_err(db_error)?;
        let rows = sqlx::query_as::<_, AddressRow>(DUPLICATES_SQL)
            .bind(limit)
            .bind(term)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(Page {
            items: rows.into_iter().map(Into::into).collect(),
            total,
            limit,
            offset: 0,
        })
    }

    /// Distinct values of the address search index ("Index wählen" dialog).
    pub async fn index_values(&self) -> Result<Vec<String>, DomainError> {
        let rows = sqlx::query_scalar::<_, String>(INDEX_VALUES_SQL)
            .fetch_all(&self.pool)
            .await
            .map_err(db_error)?;
        Ok(rows)
    }

    pub async fn delete(&self, id: i32) -> Result<(), DomainError> {
        let result = sqlx::query(DELETE_SQL)
            .bind(id)
            .execute(&self.pool)
            .await
            .map_err(db_error)?;
        if result.rows_affected() == 0 {
            return Err(DomainError::NotFound {
                entity: "Address",
                id,
            });
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_round_trip() {
        for kind in [
            AddressType::All,
            AddressType::Customers,
            AddressType::Suppliers,
        ] {
            assert_eq!(AddressType::from_key(kind.key()), Some(kind));
        }
        for mode in [
            TextMatch::Contains,
            TextMatch::StartsWith,
            TextMatch::Equals,
        ] {
            assert_eq!(TextMatch::from_key(mode.key()), Some(mode));
        }
        assert_eq!(AddressType::from_key("vielleicht"), None);
        assert_eq!(TextMatch::from_key("vielleicht"), None);
    }

    #[test]
    fn text_match_builds_like_patterns() {
        assert_eq!(TextMatch::Contains.pattern("Licht"), "%Licht%");
        assert_eq!(TextMatch::StartsWith.pattern("Licht"), "Licht%");
        assert_eq!(TextMatch::Equals.pattern("Licht"), "Licht");
    }

    #[test]
    fn list_keys_round_trip() {
        for state in [
            AddressState::All,
            AddressState::Active,
            AddressState::Inactive,
        ] {
            assert_eq!(AddressState::from_key(state.key()), Some(state));
        }
        for sort in [
            AddressSort::Type,
            AddressSort::Number,
            AddressSort::Name,
            AddressSort::Country,
            AddressSort::PostalCode,
            AddressSort::City,
        ] {
            assert_eq!(AddressSort::from_key(sort.key()), Some(sort));
        }
        assert_eq!(AddressState::from_key("vielleicht"), None);
        assert_eq!(AddressSort::from_key("vielleicht"), None);
    }

    #[test]
    fn list_defaults_to_active_addresses_sorted_by_name() {
        let list = AddressList::default();
        assert_eq!(list.state, AddressState::Active);
        assert_eq!(list.sort, AddressSort::Name);
        assert!(!list.descending);
        assert_eq!(list.limit, 50);
    }

    #[test]
    fn sort_expressions_are_fixed_column_references() {
        // A wrong column name here is what `verify-schema.mjs` fails on.
        for sort in [
            AddressSort::Type,
            AddressSort::Number,
            AddressSort::Name,
            AddressSort::Country,
            AddressSort::PostalCode,
            AddressSort::City,
        ] {
            assert!(!sort.order_expr().is_empty());
            assert!(!sort.order_expr().contains('$'));
        }
    }

    #[test]
    fn normalization_drops_empty_values_and_the_number_zero() {
        let search = AddressSearch {
            name: Some("  Theater  ".into()),
            group: Some("   ".into()),
            customer_no: Some(0),
            search_index: vec!["B".into(), "A".into(), "A".into(), "  ".into()],
            ..AddressSearch::default()
        }
        .normalized();

        assert_eq!(search.name.as_deref(), Some("Theater"));
        assert_eq!(search.group, None);
        assert_eq!(search.customer_no, None);
        assert_eq!(search.search_index, vec!["A".to_string(), "B".to_string()]);
    }

    #[test]
    fn normalization_keeps_a_real_customer_number() {
        let search = AddressSearch {
            customer_no: Some(4711),
            ..AddressSearch::default()
        }
        .normalized();
        assert_eq!(search.customer_no, Some(4711));
    }
}
