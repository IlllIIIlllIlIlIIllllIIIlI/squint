# Convention Rules (CV)

---

## CV03

**Trailing comma policy in `SELECT` clauses.**

Configure whether a trailing comma before `FROM` is forbidden (default) or required:

```toml
[rules.CV03]
select_clause_trailing_comma = "forbid"   # default
# select_clause_trailing_comma = "require"
```

**`forbid` (default)**

```sql
-- bad: trailing comma
select
    id,
    name,
from orders

-- good
select
    id,
    name
from orders
```

**`require`**

```sql
-- bad: missing trailing comma
select
    id,
    name
from orders

-- good
select
    id,
    name,
from orders
```

---

## CV04

**Consistent row-counting syntax: `COUNT(*)` vs `COUNT(1)`.**

Configure which form is required:

```toml
[rules.CV04]
prefer_count_1 = false   # default: require COUNT(*)
# prefer_count_1 = true  # require COUNT(1)
```

```sql
-- bad (with default config)
select count(1) from orders

-- good
select count(*) from orders
```

---

## CV05

**`NULL` comparisons must use `IS NULL` / `IS NOT NULL`.**

Auto-fixable. Using `= NULL` or `!= NULL` is never correct — SQL's three-valued logic
means those comparisons always return `NULL`, not `TRUE` or `FALSE`.

```sql
-- bad
select id from orders where deleted_at = null
select id from orders where status != null

-- good (auto-fixed)
select id from orders where deleted_at is null
select id from orders where status is not null
```

---

## CV10

**Identifiers must use a consistent quoting style within a file.**

Mixing quoted and unquoted forms of the same identifier in a single file is flagged.
Only the first inconsistency per identifier name is reported.

Handles double quotes (`"col"`), backticks (`` `col` ``), and brackets (`[col]`).

```sql
-- bad: 'col' used both unquoted and quoted
select col, "col" from t

-- bad: 'my_table' used both ways
select a
from my_table t
join "my_table" u on t.id = u.id

-- good: consistent throughout
select col from my_table

-- also good: consistently quoted
select "col" from "my_table"
```

> **Note:** The comparison is case-insensitive. `COL` (unquoted) and `"col"` (quoted)
> are considered the same identifier for this rule.
