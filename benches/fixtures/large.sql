-- benchmark fixture: medium (~500 lines)
-- intentional violations: ~15 CP01 uppercase keywords, ~3 AL02 implicit aliases,
--                          1 CV05 (= null), mixed indentation styles
{{ config(materialized='table') }}

with

s1_stg_events as (

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

s1_stg_sessions as (

    select
        session_id,
        user_id,
        min(event_ts) as session_start_ts,
        max(event_ts) as session_end_ts,
        count(*) as event_count,
        count(distinct event_type) as distinct_event_types,
        max(case when event_type = 'purchase' then 1 else 0 end) as had_purchase
    from s1_stg_events
    GROUP BY session_id, user_id

),

s1_user_activity as (

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
    FROM s1_stg_sessions
    group by user_id

),

s1_cohort_assignments as (

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
    from s1_user_activity

),

s1_retention_metrics as (

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
    FROM s1_cohort_assignments
    WHERE user_cohort IS NOT NULL

),

s1_funnel_steps as (

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
    from s1_stg_events e
    inner join s1_stg_sessions s
        on e.session_id = s.session_id
        and e.user_id = s.user_id

),

s1_ranked_users as (

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
    FROM s1_retention_metrics r
    LEFT JOIN (
        select
            user_id,
            max(max_funnel_step) as max_funnel_step
        from s1_funnel_steps
        GROUP BY user_id
    ) f
        on r.user_id = f.user_id

),

s1_final_metrics as (

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
    from s1_ranked_users ru
    inner join s1_user_activity ua
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
from s1_final_metrics
ORDER BY global_rank asc, user_cohort, within_bucket_rank
;

-- benchmark fixture: medium (~500 lines)
-- intentional violations: ~15 CP01 uppercase keywords, ~3 AL02 implicit aliases,
--                          1 CV05 (= null), mixed indentation styles
{{ config(materialized='table') }}

with

s2_stg_events as (

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

s2_stg_sessions as (

    select
        session_id,
        user_id,
        min(event_ts) as session_start_ts,
        max(event_ts) as session_end_ts,
        count(*) as event_count,
        count(distinct event_type) as distinct_event_types,
        max(case when event_type = 'purchase' then 1 else 0 end) as had_purchase
    from s2_stg_events
    GROUP BY session_id, user_id

),

s2_user_activity as (

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
    FROM s2_stg_sessions
    group by user_id

),

s2_cohort_assignments as (

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
    from s2_user_activity

),

s2_retention_metrics as (

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
    FROM s2_cohort_assignments
    WHERE user_cohort IS NOT NULL

),

s2_funnel_steps as (

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
    from s2_stg_events e
    inner join s2_stg_sessions s
        on e.session_id = s.session_id
        and e.user_id = s.user_id

),

s2_ranked_users as (

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
    FROM s2_retention_metrics r
    LEFT JOIN (
        select
            user_id,
            max(max_funnel_step) as max_funnel_step
        from s2_funnel_steps
        GROUP BY user_id
    ) f
        on r.user_id = f.user_id

),

s2_final_metrics as (

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
    from s2_ranked_users ru
    inner join s2_user_activity ua
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
from s2_final_metrics
ORDER BY global_rank asc, user_cohort, within_bucket_rank
;

-- benchmark fixture: medium (~500 lines)
-- intentional violations: ~15 CP01 uppercase keywords, ~3 AL02 implicit aliases,
--                          1 CV05 (= null), mixed indentation styles
{{ config(materialized='table') }}

with

s3_stg_events as (

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

s3_stg_sessions as (

    select
        session_id,
        user_id,
        min(event_ts) as session_start_ts,
        max(event_ts) as session_end_ts,
        count(*) as event_count,
        count(distinct event_type) as distinct_event_types,
        max(case when event_type = 'purchase' then 1 else 0 end) as had_purchase
    from s3_stg_events
    GROUP BY session_id, user_id

),

s3_user_activity as (

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
    FROM s3_stg_sessions
    group by user_id

),

s3_cohort_assignments as (

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
    from s3_user_activity

),

s3_retention_metrics as (

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
    FROM s3_cohort_assignments
    WHERE user_cohort IS NOT NULL

),

s3_funnel_steps as (

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
    from s3_stg_events e
    inner join s3_stg_sessions s
        on e.session_id = s.session_id
        and e.user_id = s.user_id

),

s3_ranked_users as (

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
    FROM s3_retention_metrics r
    LEFT JOIN (
        select
            user_id,
            max(max_funnel_step) as max_funnel_step
        from s3_funnel_steps
        GROUP BY user_id
    ) f
        on r.user_id = f.user_id

),

s3_final_metrics as (

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
    from s3_ranked_users ru
    inner join s3_user_activity ua
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
from s3_final_metrics
ORDER BY global_rank asc, user_cohort, within_bucket_rank
;

-- benchmark fixture: medium (~500 lines)
-- intentional violations: ~15 CP01 uppercase keywords, ~3 AL02 implicit aliases,
--                          1 CV05 (= null), mixed indentation styles
{{ config(materialized='table') }}

with

s4_stg_events as (

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

s4_stg_sessions as (

    select
        session_id,
        user_id,
        min(event_ts) as session_start_ts,
        max(event_ts) as session_end_ts,
        count(*) as event_count,
        count(distinct event_type) as distinct_event_types,
        max(case when event_type = 'purchase' then 1 else 0 end) as had_purchase
    from s4_stg_events
    GROUP BY session_id, user_id

),

s4_user_activity as (

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
    FROM s4_stg_sessions
    group by user_id

),

s4_cohort_assignments as (

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
    from s4_user_activity

),

s4_retention_metrics as (

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
    FROM s4_cohort_assignments
    WHERE user_cohort IS NOT NULL

),

s4_funnel_steps as (

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
    from s4_stg_events e
    inner join s4_stg_sessions s
        on e.session_id = s.session_id
        and e.user_id = s.user_id

),

s4_ranked_users as (

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
    FROM s4_retention_metrics r
    LEFT JOIN (
        select
            user_id,
            max(max_funnel_step) as max_funnel_step
        from s4_funnel_steps
        GROUP BY user_id
    ) f
        on r.user_id = f.user_id

),

s4_final_metrics as (

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
    from s4_ranked_users ru
    inner join s4_user_activity ua
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
from s4_final_metrics
ORDER BY global_rank asc, user_cohort, within_bucket_rank
;

-- benchmark fixture: medium (~500 lines)
-- intentional violations: ~15 CP01 uppercase keywords, ~3 AL02 implicit aliases,
--                          1 CV05 (= null), mixed indentation styles
{{ config(materialized='table') }}

with

s5_stg_events as (

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

s5_stg_sessions as (

    select
        session_id,
        user_id,
        min(event_ts) as session_start_ts,
        max(event_ts) as session_end_ts,
        count(*) as event_count,
        count(distinct event_type) as distinct_event_types,
        max(case when event_type = 'purchase' then 1 else 0 end) as had_purchase
    from s5_stg_events
    GROUP BY session_id, user_id

),

s5_user_activity as (

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
    FROM s5_stg_sessions
    group by user_id

),

s5_cohort_assignments as (

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
    from s5_user_activity

),

s5_retention_metrics as (

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
    FROM s5_cohort_assignments
    WHERE user_cohort IS NOT NULL

),

s5_funnel_steps as (

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
    from s5_stg_events e
    inner join s5_stg_sessions s
        on e.session_id = s.session_id
        and e.user_id = s.user_id

),

s5_ranked_users as (

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
    FROM s5_retention_metrics r
    LEFT JOIN (
        select
            user_id,
            max(max_funnel_step) as max_funnel_step
        from s5_funnel_steps
        GROUP BY user_id
    ) f
        on r.user_id = f.user_id

),

s5_final_metrics as (

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
    from s5_ranked_users ru
    inner join s5_user_activity ua
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
from s5_final_metrics
ORDER BY global_rank asc, user_cohort, within_bucket_rank
;

-- benchmark fixture: medium (~500 lines)
-- intentional violations: ~15 CP01 uppercase keywords, ~3 AL02 implicit aliases,
--                          1 CV05 (= null), mixed indentation styles
{{ config(materialized='table') }}

with

s6_stg_events as (

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

s6_stg_sessions as (

    select
        session_id,
        user_id,
        min(event_ts) as session_start_ts,
        max(event_ts) as session_end_ts,
        count(*) as event_count,
        count(distinct event_type) as distinct_event_types,
        max(case when event_type = 'purchase' then 1 else 0 end) as had_purchase
    from s6_stg_events
    GROUP BY session_id, user_id

),

s6_user_activity as (

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
    FROM s6_stg_sessions
    group by user_id

),

s6_cohort_assignments as (

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
    from s6_user_activity

),

s6_retention_metrics as (

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
    FROM s6_cohort_assignments
    WHERE user_cohort IS NOT NULL

),

s6_funnel_steps as (

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
    from s6_stg_events e
    inner join s6_stg_sessions s
        on e.session_id = s.session_id
        and e.user_id = s.user_id

),

s6_ranked_users as (

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
    FROM s6_retention_metrics r
    LEFT JOIN (
        select
            user_id,
            max(max_funnel_step) as max_funnel_step
        from s6_funnel_steps
        GROUP BY user_id
    ) f
        on r.user_id = f.user_id

),

s6_final_metrics as (

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
    from s6_ranked_users ru
    inner join s6_user_activity ua
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
from s6_final_metrics
ORDER BY global_rank asc, user_cohort, within_bucket_rank
;

-- benchmark fixture: medium (~500 lines)
-- intentional violations: ~15 CP01 uppercase keywords, ~3 AL02 implicit aliases,
--                          1 CV05 (= null), mixed indentation styles
{{ config(materialized='table') }}

with

s7_stg_events as (

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

s7_stg_sessions as (

    select
        session_id,
        user_id,
        min(event_ts) as session_start_ts,
        max(event_ts) as session_end_ts,
        count(*) as event_count,
        count(distinct event_type) as distinct_event_types,
        max(case when event_type = 'purchase' then 1 else 0 end) as had_purchase
    from s7_stg_events
    GROUP BY session_id, user_id

),

s7_user_activity as (

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
    FROM s7_stg_sessions
    group by user_id

),

s7_cohort_assignments as (

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
    from s7_user_activity

),

s7_retention_metrics as (

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
    FROM s7_cohort_assignments
    WHERE user_cohort IS NOT NULL

),

s7_funnel_steps as (

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
    from s7_stg_events e
    inner join s7_stg_sessions s
        on e.session_id = s.session_id
        and e.user_id = s.user_id

),

s7_ranked_users as (

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
    FROM s7_retention_metrics r
    LEFT JOIN (
        select
            user_id,
            max(max_funnel_step) as max_funnel_step
        from s7_funnel_steps
        GROUP BY user_id
    ) f
        on r.user_id = f.user_id

),

s7_final_metrics as (

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
    from s7_ranked_users ru
    inner join s7_user_activity ua
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
from s7_final_metrics
ORDER BY global_rank asc, user_cohort, within_bucket_rank
;

-- benchmark fixture: medium (~500 lines)
-- intentional violations: ~15 CP01 uppercase keywords, ~3 AL02 implicit aliases,
--                          1 CV05 (= null), mixed indentation styles
{{ config(materialized='table') }}

with

s8_stg_events as (

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

s8_stg_sessions as (

    select
        session_id,
        user_id,
        min(event_ts) as session_start_ts,
        max(event_ts) as session_end_ts,
        count(*) as event_count,
        count(distinct event_type) as distinct_event_types,
        max(case when event_type = 'purchase' then 1 else 0 end) as had_purchase
    from s8_stg_events
    GROUP BY session_id, user_id

),

s8_user_activity as (

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
    FROM s8_stg_sessions
    group by user_id

),

s8_cohort_assignments as (

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
    from s8_user_activity

),

s8_retention_metrics as (

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
    FROM s8_cohort_assignments
    WHERE user_cohort IS NOT NULL

),

s8_funnel_steps as (

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
    from s8_stg_events e
    inner join s8_stg_sessions s
        on e.session_id = s.session_id
        and e.user_id = s.user_id

),

s8_ranked_users as (

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
    FROM s8_retention_metrics r
    LEFT JOIN (
        select
            user_id,
            max(max_funnel_step) as max_funnel_step
        from s8_funnel_steps
        GROUP BY user_id
    ) f
        on r.user_id = f.user_id

),

s8_final_metrics as (

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
    from s8_ranked_users ru
    inner join s8_user_activity ua
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
from s8_final_metrics
ORDER BY global_rank asc, user_cohort, within_bucket_rank
;

-- benchmark fixture: medium (~500 lines)
-- intentional violations: ~15 CP01 uppercase keywords, ~3 AL02 implicit aliases,
--                          1 CV05 (= null), mixed indentation styles
{{ config(materialized='table') }}

with

s9_stg_events as (

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

s9_stg_sessions as (

    select
        session_id,
        user_id,
        min(event_ts) as session_start_ts,
        max(event_ts) as session_end_ts,
        count(*) as event_count,
        count(distinct event_type) as distinct_event_types,
        max(case when event_type = 'purchase' then 1 else 0 end) as had_purchase
    from s9_stg_events
    GROUP BY session_id, user_id

),

s9_user_activity as (

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
    FROM s9_stg_sessions
    group by user_id

),

s9_cohort_assignments as (

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
    from s9_user_activity

),

s9_retention_metrics as (

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
    FROM s9_cohort_assignments
    WHERE user_cohort IS NOT NULL

),

s9_funnel_steps as (

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
    from s9_stg_events e
    inner join s9_stg_sessions s
        on e.session_id = s.session_id
        and e.user_id = s.user_id

),

s9_ranked_users as (

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
    FROM s9_retention_metrics r
    LEFT JOIN (
        select
            user_id,
            max(max_funnel_step) as max_funnel_step
        from s9_funnel_steps
        GROUP BY user_id
    ) f
        on r.user_id = f.user_id

),

s9_final_metrics as (

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
    from s9_ranked_users ru
    inner join s9_user_activity ua
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
from s9_final_metrics
ORDER BY global_rank asc, user_cohort, within_bucket_rank
;

-- benchmark fixture: medium (~500 lines)
-- intentional violations: ~15 CP01 uppercase keywords, ~3 AL02 implicit aliases,
--                          1 CV05 (= null), mixed indentation styles
{{ config(materialized='table') }}

with

s10_stg_events as (

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

s10_stg_sessions as (

    select
        session_id,
        user_id,
        min(event_ts) as session_start_ts,
        max(event_ts) as session_end_ts,
        count(*) as event_count,
        count(distinct event_type) as distinct_event_types,
        max(case when event_type = 'purchase' then 1 else 0 end) as had_purchase
    from s10_stg_events
    GROUP BY session_id, user_id

),

s10_user_activity as (

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
    FROM s10_stg_sessions
    group by user_id

),

s10_cohort_assignments as (

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
    from s10_user_activity

),

s10_retention_metrics as (

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
    FROM s10_cohort_assignments
    WHERE user_cohort IS NOT NULL

),

s10_funnel_steps as (

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
    from s10_stg_events e
    inner join s10_stg_sessions s
        on e.session_id = s.session_id
        and e.user_id = s.user_id

),

s10_ranked_users as (

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
    FROM s10_retention_metrics r
    LEFT JOIN (
        select
            user_id,
            max(max_funnel_step) as max_funnel_step
        from s10_funnel_steps
        GROUP BY user_id
    ) f
        on r.user_id = f.user_id

),

s10_final_metrics as (

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
    from s10_ranked_users ru
    inner join s10_user_activity ua
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
from s10_final_metrics
ORDER BY global_rank asc, user_cohort, within_bucket_rank
;
