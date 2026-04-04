# Ambiguous Rules (AM)

---

## AM01

**`SELECT DISTINCT` with `GROUP BY` is redundant.**

`DISTINCT` has no effect when results are already deduplicated by `GROUP BY`. One of them
should be removed.

```sql
-- bad: DISTINCT is redundant
select distinct status, count(*)
from orders
group by status

-- good
select status, count(*)
from orders
group by status
```

---

## AM02

**`UNION` must be followed by `ALL` or `DISTINCT`.**

Bare `UNION` is ambiguous — in most databases it implies `DISTINCT`, but this is easily
confused with `UNION ALL`. Be explicit.

```sql
-- bad: ambiguous UNION
select a from t
union
select a from u

-- good
select a from t
union all
select a from u

-- also good
select a from t
union distinct
select a from u
```

---

## AM05

**Implicit comma joins in `FROM` are forbidden; use explicit `JOIN` syntax.**

Comma-separated tables in `FROM` are an old SQL syntax for cross joins / implicit inner
joins. They are harder to read and error-prone.

```sql
-- bad: implicit join
select a.id, b.name
from orders a, customers b
where a.customer_id = b.id

-- good
select a.id, b.name
from orders a
inner join customers b on a.customer_id = b.id
```

---

## AM06

**`GROUP BY` / `ORDER BY` must use a consistent reference style.**

Configure the required style:

```toml
[rules.AM06]
group_by_and_order_by_style = "explicit"    # default
# group_by_and_order_by_style = "implicit"
# group_by_and_order_by_style = "consistent"
```

| Style | Meaning |
|---|---|
| `explicit` (default) | All references must be column names — positional numbers are flagged |
| `implicit` | All references must be positional numbers — column names are flagged |
| `consistent` | Either style is allowed, but mixing within one clause is flagged |

**`explicit` (default):**

```sql
-- bad: positional reference
select status, count(*) from orders group by 1

-- good
select status, count(*) from orders group by status
```

**`implicit`:**

```sql
-- bad: named reference
select status, count(*) from orders group by status

-- good
select status, count(*) from orders group by 1
```
