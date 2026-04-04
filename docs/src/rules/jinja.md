# Jinja Rules (JJ)

---

## JJ01

**Jinja tags must have single-space padding inside their delimiters.**

This applies to both expression tags (`{{ }}`) and statement tags (`{% %}`).

```sql
-- bad: no spaces inside delimiters
select {{my_col}} from {{ref('my_model')}}
{% if condition %}

-- good
select {{ my_col }} from {{ ref('my_model') }}
{% if condition %}
```

### Details

| Pattern | Example | Status |
|---|---|---|
| Expression, no padding | `{{col}}` | Flagged |
| Expression, correct padding | `{{ col }}` | OK |
| Statement, no padding | `{%if cond%}` | Flagged |
| Statement, correct padding | `{% if cond %}` | OK |
| Strip whitespace modifier | `{%- if cond -%}` | OK |

The rule checks the raw tag text including delimiters. It does not parse the Jinja
expression content.
