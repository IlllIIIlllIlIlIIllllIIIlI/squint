# Aliasing Rules (AL)

---

## AL02

**Column aliases must use the explicit `AS` keyword.**

```sql
-- bad: implicit alias (no AS)
select id order_id, name customer_name from orders

-- good
select id as order_id, name as customer_name from orders
```

---

## AL03

**Expressions in `SELECT` must have an alias.**

Bare column names do not need an alias. Function calls, operators, and multi-token
expressions do.

```sql
-- bad
select count(id), price * 1.1, coalesce(name, 'unknown')
from orders

-- good
select
    count(id) as order_count,
    price * 1.1 as price_with_tax,
    coalesce(name, 'unknown') as display_name
from orders
```

---

## AL04

**Table aliases must be unique within a query.**

```sql
-- bad: 'o' used twice
select o.id, o.name
from orders o
join order_items o on o.order_id = o.id

-- good
select o.id, oi.name
from orders o
join order_items oi on o.id = oi.order_id
```

---

## AL05

**Table aliases that are defined but never referenced.**

```sql
-- bad: alias 'o' defined but never used
select id, name from orders o

-- good: alias used
select o.id, o.name from orders o

-- also good: no alias at all
select id, name from orders
```

---

## AL06

**Table alias length must be within configured bounds.**

Configure minimum and maximum alias length:

```toml
[rules.AL06]
min_alias_length = 1   # default: 1  (0 = no minimum)
max_alias_length = 0   # default: 0  (0 = no maximum)
```

Example with `min_alias_length = 3`:

```sql
-- bad: alias 'o' is too short (< 3)
select o.id from orders o

-- good
select ord.id from orders ord
```

---

## AL08

**Column aliases in `SELECT` must be unique (case-insensitive).**

```sql
-- bad: 'id' used twice as alias
select
    order_id as id,
    customer_id as id
from orders

-- good
select
    order_id,
    customer_id
from orders
```

---

## AL09

**A column must not be aliased to itself (`col AS col`).**

```sql
-- bad: redundant self-alias
select id as id, name as name from orders

-- good
select id, name from orders
```
