# Structure Rules (ST)

---

## ST03

**CTEs that are defined but never referenced.**

An unused CTE is dead code — it adds query planning overhead and confuses readers.

```sql
-- bad: 'unused_cte' is never referenced
with
    used_cte as (select id from orders),
    unused_cte as (select id from customers)
select id from used_cte

-- good
with used_cte as (
    select id from orders
)
select id from used_cte
```

---

## ST08

**`COUNT(DISTINCT *)` is not valid SQL.**

`DISTINCT *` inside `COUNT` is not standard SQL and is rejected by most databases.
Use `COUNT(DISTINCT col)` with an explicit column name instead.

```sql
-- bad
select count(distinct *) from orders

-- good
select count(distinct id) from orders
```
