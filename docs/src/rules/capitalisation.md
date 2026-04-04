# Capitalisation Rules (CP)

All capitalisation rules enforce **lowercase**. This matches the style used by
[sqlfmt](https://sqlfmt.com/) and is the default for most dbt projects. All five rules
are auto-fixable with `--fix`.

---

## CP01

**Keywords must be lowercase.**

Covers: `SELECT`, `FROM`, `WHERE`, `GROUP BY`, `ORDER BY`, `HAVING`, `LIMIT`, `JOIN`,
`UNION`, `AND`, `OR`, `NOT`, `IN`, `IS`, `LIKE`, `BETWEEN`, `CASE`, `WHEN`, `THEN`,
`ELSE`, `END`, `AS`, `DISTINCT`, `ALL`, `ON`, `WITH`, and all other SQL keywords.

```sql
-- bad
SELECT id, name
FROM orders
WHERE status = 'active'

-- good
select id, name
from orders
where status = 'active'
```

---

## CP02

**Unquoted identifiers must be lowercase.**

Applies to table names, column names, CTE names, and aliases that are written without
quotes. Quoted identifiers (e.g. `"MyTable"`) are exempt.

```sql
-- bad
select OrderId, CustomerName
from Orders o

-- good
select orderid, customername
from orders o
```

---

## CP03

**Function names must be lowercase.**

Applies to any name token immediately followed by `(`.

```sql
-- bad
SELECT COUNT(id), COALESCE(name, 'unknown'), UPPER(email)
FROM customers

-- good
select count(id), coalesce(name, 'unknown'), upper(email)
from customers
```

---

## CP04

**Boolean and null literals must be lowercase.**

Covers `TRUE`, `FALSE`, and `NULL`.

```sql
-- bad
select id
from orders
where is_active = TRUE and deleted_at IS NULL

-- good
select id
from orders
where is_active = true and deleted_at is null
```

---

## CP05

**Data type names must be lowercase.**

Covers: `int`, `integer`, `bigint`, `smallint`, `float`, `double`, `decimal`, `numeric`,
`varchar`, `char`, `text`, `boolean`, `bool`, `date`, `time`, `timestamp`, `timestamptz`,
`json`, `jsonb`, `uuid`, and others.

```sql
-- bad (in a CAST expression)
select cast(id as VARCHAR(36)), cast(amount as DECIMAL(10, 2))
from orders

-- good
select cast(id as varchar(36)), cast(amount as decimal(10, 2))
from orders
```
