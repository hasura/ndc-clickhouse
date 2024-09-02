SELECT
  toJSONString(
    groupArray(
      cast(
        "_rowset"."_rowset",
        'Tuple(rows Array(Tuple("revenue" UInt64)))'
      )
    )
  ) AS "rowsets"
FROM
  (
    SELECT
      tuple(groupArray(tuple("_row"."_field_revenue"))) AS "_rowset"
    FROM
      (
        SELECT
          "_origin"."revenue" AS "_field_revenue"
        FROM
          (
            SELECT
              sum(LO_EXTENDEDPRICE * LO_DISCOUNT) AS revenue
            FROM
              star.lineorder_flat
            WHERE
              toYear(LO_ORDERDATE) = 1993
              AND LO_DISCOUNT BETWEEN 1 AND 3
              AND LO_QUANTITY < 25
          ) AS "_origin"
      ) AS "_row"
  ) AS "_rowset" FORMAT TabSeparatedRaw;