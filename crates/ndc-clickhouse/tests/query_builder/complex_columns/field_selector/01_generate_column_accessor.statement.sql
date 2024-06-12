SELECT
  toJSONString(
    groupArray(
      cast(
        "_rowset"."_rowset",
        'Tuple(rows Array(Tuple("field1" Array(Tuple("subfield1" String)))))'
      )
    )
  ) AS "rowsets"
FROM
  (
    SELECT
      tuple(groupArray(tuple("_row"."_field_field1"))) AS "_rowset"
    FROM
      (
        SELECT
          arrayMap(
            (_value) -> tuple(_value."field1"),
            "_origin"."ColumnB"
          ) AS "_field_field1"
        FROM
          "Schema1"."Table1" AS "_origin"
      ) AS "_row"
  ) AS "_rowset" FORMAT TabSeparatedRaw;