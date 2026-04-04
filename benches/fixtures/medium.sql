-- benchmark fixture: medium (~500 lines)
-- intentional violations: ~15 CP01 uppercase keywords, ~3 AL02 implicit aliases,
--                          1 CV05 (= null), mixed indentation styles
{{ config(materialized='table') }}

with

stg_events as (

    SELECT
        event_id,
        user_id,
        session_id,
        event_type,
        event_ts,
        page_url,
        referrer_url,
        device_type,
        country_code,
        properties
    FROM {{ ref('raw_events') }}
    {% if var('start_date', none) is not none %}
    WHERE event_ts >= '{{ var("start_date") }}'
    {% endif %}

),

stg_sessions as (

    select
        session_id,
        user_id,
        min(event_ts) as session_start_ts,
        max(event_ts) as session_end_ts,
        count(*) as event_count,
        count(distinct event_type) as distinct_event_types,
        max(case when event_type = 'purchase' then 1 else 0 end) as had_purchase
    from stg_events
    GROUP BY session_id, user_id

),

user_activity as (

    select
        user_id,
        count(distinct session_id) as total_sessions,
        count(distinct case when had_purchase = 1 then session_id end) as purchase_sessions,
        sum(event_count) as total_events,
        min(session_start_ts) as first_seen_ts,
        max(session_end_ts) as last_seen_ts,
        sum(sum(event_count)) over (
            partition by user_id
            order by min(session_start_ts)
            rows between unbounded preceding and current row
        ) as cumulative_events
    FROM stg_sessions
    group by user_id

),

cohort_assignments as (

    select
        user_id,
        first_seen_ts,
        last_seen_ts,
        total_sessions,
        purchase_sessions,
        total_events,
        cumulative_events,
        case
            when total_sessions >= 10 and purchase_sessions >= 3 then 'power_buyer'
            when total_sessions >= 5  and purchase_sessions >= 1 then 'regular_buyer'
            when total_sessions >= 3  then 'engaged_browser'
            when total_sessions >= 1  then 'casual_visitor'
            else 'one_time'
        end as user_cohort,
        case
            when datediff('day', first_seen_ts, last_seen_ts) >= 90 then 'long_term'
            when datediff('day', first_seen_ts, last_seen_ts) >= 30 then 'medium_term'
            else 'short_term'
        end as tenure_bucket
    from user_activity

),

retention_metrics as (

    select
        user_id,
        user_cohort,
        tenure_bucket,
        first_seen_ts,
        last_seen_ts,
        total_sessions,
        purchase_sessions,
        lag(total_sessions, 1) over (
            partition by user_cohort
            order by first_seen_ts
        ) as prev_total_sessions,
        lead(total_sessions, 1) over (
            partition by user_cohort
            order by first_seen_ts
        ) as next_total_sessions,
        row_number() over (
            partition by user_cohort
            order by total_sessions desc, first_seen_ts asc
        ) as cohort_rank
    FROM cohort_assignments
    WHERE user_cohort IS NOT NULL

),

funnel_steps as (

    select
        e.user_id,
        e.session_id,
        e.event_type,
        e.event_ts,
        s.had_purchase,
        s.event_count,
        case
            when e.event_type = 'page_view'    then 1
            when e.event_type = 'product_view' then 2
            when e.event_type = 'add_to_cart'  then 3
            when e.event_type = 'checkout'     then 4
            when e.event_type = 'purchase'     then 5
            else 0
        end as funnel_step,
        max(case
            when e.event_type = 'page_view'    then 1
            when e.event_type = 'product_view' then 2
            when e.event_type = 'add_to_cart'  then 3
            when e.event_type = 'checkout'     then 4
            when e.event_type = 'purchase'     then 5
            else 0
        end) over (partition by e.session_id) as max_funnel_step
    from stg_events e
    inner join stg_sessions s
        on e.session_id = s.session_id
        and e.user_id = s.user_id

),

ranked_users as (

    select
        r.user_id,
        r.user_cohort,
        r.tenure_bucket,
        r.cohort_rank,
        r.prev_total_sessions,
        r.next_total_sessions,
        f.max_funnel_step,
        rank() over (
            partition by r.user_cohort, r.tenure_bucket
            order by r.total_sessions desc
        ) as within_bucket_rank,
        dense_rank() over (
            order by r.total_sessions desc
        ) as global_rank,
        percent_rank() over (
            partition by r.user_cohort
            order by r.total_sessions desc
        ) as cohort_pct_rank
    FROM retention_metrics r
    LEFT JOIN (
        select
            user_id,
            max(max_funnel_step) as max_funnel_step
        from funnel_steps
        GROUP BY user_id
    ) f
        on r.user_id = f.user_id

),

final_metrics as (

    select
        ru.user_id,
        ru.user_cohort,
        ru.tenure_bucket,
        ru.cohort_rank,
        ru.within_bucket_rank,
        ru.global_rank,
        ru.cohort_pct_rank,
        ru.max_funnel_step,
        ru.prev_total_sessions,
        ru.next_total_sessions,
        ua.total_sessions,
        ua.purchase_sessions,
        ua.total_events,
        ua.cumulative_events,
        ua.first_seen_ts,
        ua.last_seen_ts,
        case
            when ru.user_cohort = 'power_buyer'    then 5
            when ru.user_cohort = 'regular_buyer'  then 4
            when ru.user_cohort = 'engaged_browser' then 3
            when ru.user_cohort = 'casual_visitor' then 2
            else 1
        end as cohort_score,
        coalesce(ru.max_funnel_step, 0) as funnel_depth,
        case
            when ua.purchase_sessions > 0
            then cast(ua.purchase_sessions as float) / ua.total_sessions
            else 0.0
        end as purchase_rate
    from ranked_users ru
    inner join user_activity ua
        on ru.user_id = ua.user_id
    WHERE ru.cohort_rank <= 1000

)

SELECT
    user_id,
    user_cohort,
    tenure_bucket,
    cohort_rank,
    within_bucket_rank,
    global_rank,
    cohort_pct_rank,
    max_funnel_step,
    funnel_depth,
    total_sessions,
    purchase_sessions,
    purchase_rate,
    total_events,
    cumulative_events,
    cohort_score,
    first_seen_ts,
    last_seen_ts,
    current_timestamp() as loaded_at
from final_metrics
ORDER BY global_rank asc, user_cohort, within_bucket_rank
