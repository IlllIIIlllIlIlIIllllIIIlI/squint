-- benchmark fixture: small (~50 lines)
-- intentional violations: uppercase keywords (CP01), implicit alias (AL02)
{{ config(materialized='table') }}

with

raw_orders as (

    SELECT
        order_id,
        customer_id,
        order_date,
        amount,
        status
    FROM {{ ref('stg_orders') }}
    WHERE status != 'cancelled'

),

customer_totals as (

    select
        customer_id,
        count(*) order_count,
        sum(amount) as total_amount,
        min(order_date) as first_order_date,
        max(order_date) as last_order_date
    from raw_orders
    GROUP BY customer_id

)

select
    customer_id,
    order_count,
    total_amount,
    first_order_date,
    last_order_date
from customer_totals
ORDER BY total_amount desc
