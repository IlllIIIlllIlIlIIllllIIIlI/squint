# All Rules

35 rules across 8 categories. Fixable rules are marked **✓** — run `--fix` to apply them
automatically.

## Capitalisation

| ID | Description | Fixable |
|---|---|---|
| [CP01](capitalisation.md#cp01) | Keywords must be lowercase | ✓ |
| [CP02](capitalisation.md#cp02) | Unquoted identifiers must be lowercase | ✓ |
| [CP03](capitalisation.md#cp03) | Function names must be lowercase | ✓ |
| [CP04](capitalisation.md#cp04) | Boolean/null literals must be lowercase | ✓ |
| [CP05](capitalisation.md#cp05) | Data type names must be lowercase | ✓ |

## Layout

| ID | Description | Fixable |
|---|---|---|
| [LT01](layout.md#lt01) | No space before comma; no consecutive mid-line spaces; no space between function and `(` | ✓ |
| [LT02](layout.md#lt02) | Indentation must use spaces, multiple of 4 | |
| [LT03](layout.md#lt03) | No trailing whitespace on lines | ✓ |
| [LT05](layout.md#lt05) | Lines must not exceed `max_line_length` (default 120) | |
| [LT06](layout.md#lt06) | No space between function name and `(` | ✓ |
| [LT07](layout.md#lt07) | CTE closing `)` must be on its own line | |
| [LT08](layout.md#lt08) | Blank line required after each CTE closing `)` | |
| [LT09](layout.md#lt09) | Clauses in standard order: `SELECT → FROM → WHERE → GROUP BY → HAVING → ORDER BY → LIMIT` | |
| [LT10](layout.md#lt10) | `DISTINCT`/`ALL` must be on the same line as `SELECT` | |
| [LT11](layout.md#lt11) | Set operators (`UNION`, `INTERSECT`, `EXCEPT`) must be on their own line | |
| [LT12](layout.md#lt12) | File must end with exactly one trailing newline | ✓ |

## Convention

| ID | Description | Fixable |
|---|---|---|
| [CV03](convention.md#cv03) | Trailing comma policy in `SELECT` clauses | |
| [CV04](convention.md#cv04) | Consistent row-counting syntax: `COUNT(*)` vs `COUNT(1)` | |
| [CV05](convention.md#cv05) | `NULL` comparisons must use `IS NULL` / `IS NOT NULL` | ✓ |
| [CV10](convention.md#cv10) | Identifiers must use a consistent quoting style within a file | |

## Aliasing

| ID | Description | Fixable |
|---|---|---|
| [AL02](aliasing.md#al02) | Column aliases must use explicit `AS` keyword | |
| [AL03](aliasing.md#al03) | Expressions in `SELECT` must have an alias | |
| [AL04](aliasing.md#al04) | Table aliases must be unique within a query | |
| [AL05](aliasing.md#al05) | Table aliases that are defined but never referenced | |
| [AL06](aliasing.md#al06) | Table alias length must be within configured bounds | |
| [AL08](aliasing.md#al08) | Column aliases in `SELECT` must be unique (case-insensitive) | |
| [AL09](aliasing.md#al09) | A column must not be aliased to itself (`col AS col`) | |

## Ambiguous

| ID | Description | Fixable |
|---|---|---|
| [AM01](ambiguous.md#am01) | `SELECT DISTINCT` with `GROUP BY` is redundant | |
| [AM02](ambiguous.md#am02) | `UNION` must be followed by `ALL` or `DISTINCT` | |
| [AM05](ambiguous.md#am05) | Implicit comma joins are forbidden; use explicit `JOIN` | |
| [AM06](ambiguous.md#am06) | `GROUP BY` / `ORDER BY` must use a consistent reference style | |

## References

| ID | Description | Fixable |
|---|---|---|
| [RF01](references.md#rf01) | Qualified column references must use a known table alias | |
| [RF02](references.md#rf02) | Wildcard (`SELECT *`) is not allowed; list columns explicitly | |

## Structure

| ID | Description | Fixable |
|---|---|---|
| [ST03](structure.md#st03) | CTEs that are defined but never referenced | |
| [ST08](structure.md#st08) | `COUNT(DISTINCT *)` is not valid SQL | |

## Jinja

| ID | Description | Fixable |
|---|---|---|
| [JJ01](jinja.md#jj01) | Jinja tags must have single-space padding inside delimiters | |
