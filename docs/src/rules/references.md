# References Rules (RF)

---

## RF01

**Qualified column references must use a known table alias.**

When a column is qualified (`alias.column`), the qualifier must be an alias (or bare
table name) that was defined in the `FROM` clause of the same query.

```sql
-- bad: 'x' is not a known alias
select x.id from orders o where x.status = 'active'

-- good
select o.id from orders o where o.status = 'active'
```

> **Note:** RF01 is best-effort. It may produce false negatives for complex subquery
> and CTE patterns where aliases are defined in nested scopes.

---

## RF02

**Wildcard column references are not allowed; list columns explicitly.**

`SELECT *` and `SELECT table.*` are flagged. In production SQL, implicit column lists
cause breakage when upstream tables change.

```sql
-- bad
select * from orders
select o.* from orders o

-- good: list columns explicitly
select id, status, created_at from orders
```

**Exceptions — not flagged:**

- `COUNT(*)` — the star is inside function parens, not a column reference
- `COUNT(DISTINCT *)` — same
- Arithmetic: `select a * b from t` — star preceded by a value token is an operator

```sql
-- ok: COUNT(*) is not a wildcard column reference
select count(*) from orders

-- ok: arithmetic multiplication
select price * quantity as total from order_items
```
