# Suppression

Two mechanisms let you opt out of linting for specific lines or blocks.

## `-- noqa` — per-line suppression

Add `-- noqa` to the end of a line to suppress all rule violations on that line:

```sql
SELECT A, B, C  -- noqa
FROM MY_TABLE   -- noqa
```

To suppress only specific rules, list them after a colon:

```sql
SELECT A  -- noqa: CP01
FROM T    -- noqa: CP01, LT05
```

Rule IDs are case-insensitive: `-- noqa: cp01` and `-- noqa: CP01` are equivalent.

`-- noqa` also suppresses **auto-fixes** — a `--fix` run will not modify a line with
a bare `-- noqa` annotation.

### Combining multiple rules

```sql
SELECT A  -- noqa: CP01, LT05, AL02
```

### When to use it

Use `-- noqa` for intentional one-off exceptions: a line that must be uppercase for a
specific reason, a known long line that can't be shortened, etc. For recurring patterns,
prefer a config override or `-- fmt: off` blocks.

---

## `-- fmt: off` / `-- fmt: on` — block suppression

Suppress all rules (violations and fixes) for a block of SQL:

```sql
-- fmt: off
SELECT A,      B,
       C       -- hand-crafted alignment
FROM T
-- fmt: on

select d from u  -- linting resumes here
```

### Inline `-- fmt: off`

When `-- fmt: off` appears after SQL on the same line, it suppresses **that line only**:

```sql
select a from t  -- fmt: off
select b from u  -- this line is linted normally
```

### To end of file

A standalone `-- fmt: off` with no matching `-- fmt: on` suppresses from that line to
the end of the file:

```sql
select a from t

-- fmt: off
-- Everything below this line is suppressed
SELECT B FROM U
```

---

## Interaction between mechanisms

`-- noqa` and `-- fmt: off` are independent. A line inside a `-- fmt: off` block that
also has `-- noqa` is doubly suppressed — both mechanisms apply, but the result is the
same: no violations reported, no fixes applied.
