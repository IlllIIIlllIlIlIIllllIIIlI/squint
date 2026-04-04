# Layout Rules (LT)

---

## LT01

**No space before comma; no consecutive mid-line spaces; no space between function name and `(`.**

Auto-fixable. Three related spacing issues in one rule:

```sql
-- bad: space before comma
select a , b , c from t

-- good
select a, b, c from t
```

```sql
-- bad: consecutive spaces mid-line
select  a  from  t

-- good
select a from t
```

```sql
-- bad: space between function and (
select count (*), coalesce (name, 'x') from t

-- good
select count(*), coalesce(name, 'x') from t
```

LT01 does not touch indentation (spaces at the start of a line). That is LT02's job.

---

## LT02

**Indentation must use spaces (no tabs) and be a multiple of 4.**

```sql
-- bad: tab indentation
select
	id,
	name
from t

-- bad: 2-space indentation
select
  id,
  name
from t

-- good
select
    id,
    name
from t
```

---

## LT03

**Lines must not have trailing whitespace (spaces or tabs before the newline).**

Auto-fixable. Trailing whitespace is invisible in most editors but causes noisy diffs
and is flagged by many code review tools.

---

## LT05

**Lines must not exceed `max_line_length` characters (default: 120).**

Configure in `squint.toml`:

```toml
[rules.LT05]
max_line_length = 100
```

Or override on the command line:

```bash
squint --max-line-length 100 models/
```

---

## LT06

**No space between a function name and `(`.**

Auto-fixable. Equivalent to the function-paren check in LT01, but reported separately
with a more targeted message.

```sql
-- bad
select count (id) from t

-- good
select count(id) from t
```

---

## LT07

**CTE closing `)` must be on its own line.**

```sql
-- bad
with cte as (select id from t)
select id from cte

-- good
with cte as (
    select id from t
)
select id from cte
```

---

## LT08

**A blank line is required after each CTE closing `)`.**

```sql
-- bad
with cte as (
    select id from t
)
select id from cte   -- no blank line after the closing )

-- good
with cte as (
    select id from t
)

select id from cte
```

---

## LT09

**SQL clauses must appear in the standard order.**

Expected order: `SELECT → FROM → WHERE → GROUP BY → HAVING → ORDER BY → LIMIT`

All clauses are optional; the rule only fires when a clause appears *after* one with a
higher rank in the expected sequence.

```sql
-- bad: WHERE before FROM
select a
where a = 1
from t

-- bad: ORDER BY before GROUP BY
select a, count(*)
from t
group by a
order by a
having count(*) > 1   -- HAVING after ORDER BY is out of order

-- good
select a, count(*)
from t
where a is not null
group by a
having count(*) > 1
order by a
limit 100
```

Subqueries and CTEs each have their own independent clause order context.
`UNION ALL` resets the context for each SELECT.

---

## LT10

**`DISTINCT` / `ALL` must be on the same line as `SELECT`.**

```sql
-- bad
select
distinct a, b
from t

-- good
select distinct a, b
from t
```

---

## LT11

**Set operators (`UNION`, `INTERSECT`, `EXCEPT`) must be on their own line.**

```sql
-- bad
select a from t union all select a from u

-- good
select a from t
union all
select a from u
```

---

## LT12

**File must end with exactly one trailing newline.**

Auto-fixable. Files with no trailing newline get one added; files with multiple trailing
newlines have the extras removed.
